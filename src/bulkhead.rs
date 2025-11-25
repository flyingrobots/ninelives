//! Bulkhead implementation for concurrency limiting.
//!
//! A bulkhead caps concurrent operations to protect downstream services and bound resource usage.
//! This implementation is non-blocking: when no permits are available it rejects immediately
//! (`ResilienceError::Bulkhead`) rather than queuing. Permits are released when the wrapped
//! operation finishes.
//!
//! Example (brief):
//! ```rust
//! use ninelives::{BulkheadPolicy, ResilienceError};
//!
//! # tokio::runtime::Runtime::new().unwrap().block_on(async {
//! let bulkhead = BulkheadPolicy::new(1).unwrap();
//! let (hold_tx, hold_rx) = tokio::sync::oneshot::channel();
//! let (started_tx, started_rx) = tokio::sync::oneshot::channel();
//!
//! // Hold the single permit
//! let bh_handle = bulkhead.clone();
//! let first_task = tokio::spawn(async move {
//!     bh_handle
//!         .execute(|| async move {
//!             let _ = started_tx.send(());
//!             let _ = hold_rx.await;
//!             Ok::<_, ResilienceError<std::io::Error>>(())
//!         })
//!         .await
//! });
//! tokio::task::yield_now().await; // ensure the first task acquires the permit
//! let _ = started_rx.await;
//!
//! // Compete for the already-held permit
//! let second_res = bulkhead
//!     .execute(|| async { Ok::<_, ResilienceError<std::io::Error>>(()) })
//!     .await;
//! assert!(second_res.unwrap_err().is_bulkhead());
//! let _ = hold_tx.send(());
//! assert!(first_task.await.unwrap().is_ok());
//! # });
//! ```
//!
//! For a fuller concurrent walkthrough, see `examples/bulkhead_concurrency.rs`.

use crate::ResilienceError;
use futures::future::BoxFuture;
use std::future::Future;
use std::sync::Arc;
use tokio::sync::{Semaphore, TryAcquireError};
use tower_layer::Layer;
use tower_service::Service;

#[derive(Debug, Clone)]
/// Concurrency-limiting bulkhead. Clones share the same underlying semaphore via `Arc`, so they
/// observe and affect the same in-flight count. `max_concurrent` is the configured permit ceiling.
pub struct BulkheadPolicy {
    semaphore: Arc<Semaphore>,
    /// Mirrors the initial semaphore capacity; used only for reporting.
    max_concurrent: usize,
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
/// Errors produced while configuring a bulkhead (e.g., invalid permit counts).
pub enum BulkheadError {
    /// `max_concurrent` was zero (invalid).
    InvalidMaxConcurrent {
        /// The invalid max_concurrent value supplied.
        provided: usize,
    },
}

impl std::fmt::Display for BulkheadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BulkheadError::InvalidMaxConcurrent { provided } => {
                write!(f, "bulkhead max_concurrent must be > 0 (got {})", provided)
            }
        }
    }
}

impl std::error::Error for BulkheadError {}

/// Large but finite permit count used to approximate "unlimited".
pub const UNLIMITED_PERMITS: usize = Semaphore::MAX_PERMITS;

impl BulkheadPolicy {
    /// Create a bulkhead with the given maximum concurrent permits.
    /// Returns `Err` if `max_concurrent` is zero.
    pub fn new(max_concurrent: usize) -> Result<Self, BulkheadError> {
        if max_concurrent == 0 {
            return Err(BulkheadError::InvalidMaxConcurrent { provided: max_concurrent });
        }

        Ok(Self { semaphore: Arc::new(Semaphore::new(max_concurrent)), max_concurrent })
    }

    /// Construct an effectively unlimited bulkhead using `UNLIMITED_PERMITS` (derived from
    /// `Semaphore::MAX_PERMITS`). Operations still reject immediately if all permits are in use.
    pub fn unlimited() -> Self {
        // Safe: constant respects semaphore maximum.
        Self::new(UNLIMITED_PERMITS).unwrap()
    }

    /// Maximum configured concurrent permits.
    pub fn max_concurrent(&self) -> usize {
        self.max_concurrent
    }

    /// Best-effort current available permits (may be stale due to races).
    pub fn available_permits(&self) -> usize {
        self.semaphore.available_permits()
    }

    /// Execute an operation with bulkhead protection. Non-blocking: if no permits are available
    /// the call is rejected immediately with `ResilienceError::Bulkhead`. If the semaphore has been
    /// closed, returns `ResilienceError::BulkheadClosed`. Permits are released when the operation
    /// future completes. The reported `in_flight` is a best-effort snapshot and may be stale due to
    /// races.
    pub async fn execute<T, E, Fut, Op>(&self, operation: Op) -> Result<T, ResilienceError<E>>
    where
        T: Send,
        E: std::error::Error + Send + Sync + 'static,
        Fut: Future<Output = Result<T, ResilienceError<E>>> + Send,
        Op: FnOnce() -> Fut + Send,
    {
        let _permit = match self.semaphore.try_acquire() {
            Ok(p) => p,
            Err(tokio::sync::TryAcquireError::NoPermits) => {
                let available = self.semaphore.available_permits(); // best-effort snapshot
                let in_flight = self.max_concurrent.saturating_sub(available);
                return Err(ResilienceError::Bulkhead { in_flight, max: self.max_concurrent });
            }
            Err(tokio::sync::TryAcquireError::Closed) => {
                return Err(ResilienceError::BulkheadClosed);
            }
        };

        // permit released on scope exit
        operation().await
    }
}

/// Tower-native bulkhead layer that limits concurrent in-flight requests.
///
/// Each call to `layer()` creates a new `BulkheadService` with its own `Arc<Semaphore>`;
/// limits are therefore per-service instance. To share concurrency limits across multiple
/// services, store an `Arc<Semaphore>` inside the layer and clone it into each service.
#[derive(Debug, Clone)]
pub struct BulkheadLayer {
    max_concurrent: usize,
}

impl BulkheadLayer {
    /// Create a bulkhead layer; returns error if `max_concurrent` is zero.
    pub fn new(max_concurrent: usize) -> Result<Self, BulkheadError> {
        BulkheadPolicy::new(max_concurrent)
            .map(|p| BulkheadLayer { max_concurrent: p.max_concurrent })
    }
}

/// Service produced by [`BulkheadLayer`]; enforces permit limits.
#[derive(Debug, Clone)]
pub struct BulkheadService<S> {
    semaphore: Arc<Semaphore>,
    max_concurrent: usize,
    inner: S,
}

impl<S> BulkheadService<S> {
    fn new(inner: S, max_concurrent: usize) -> Self {
        Self { semaphore: Arc::new(Semaphore::new(max_concurrent)), max_concurrent, inner }
    }
}

impl<S, Request> Service<Request> for BulkheadService<S>
where
    S: Service<Request> + Clone + Send + 'static,
    S::Future: Send + 'static,
    Request: Send + 'static,
    S::Error: std::error::Error + Send + Sync + 'static,
    S::Response: Send + 'static,
{
    type Response = S::Response;
    type Error = ResilienceError<S::Error>;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx).map_err(ResilienceError::Inner)
    }

    fn call(&mut self, req: Request) -> Self::Future {
        let semaphore = self.semaphore.clone();
        let mut inner = self.inner.clone();
        let max = self.max_concurrent;
        Box::pin(async move {
            let _permit = match semaphore.try_acquire_owned() {
                Ok(p) => p,
                Err(TryAcquireError::NoPermits) => {
                    return Err(ResilienceError::Bulkhead { in_flight: max, max });
                }
                Err(TryAcquireError::Closed) => {
                    return Err(ResilienceError::BulkheadClosed);
                }
            };
            let result = inner.call(req).await;
            result.map_err(ResilienceError::Inner)
        })
    }
}

impl<S> Layer<S> for BulkheadLayer {
    type Service = BulkheadService<S>;
    fn layer(&self, service: S) -> Self::Service {
        BulkheadService::new(service, self.max_concurrent)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::sync::oneshot;

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct TestError(String);

    impl std::fmt::Display for TestError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "TestError: {}", self.0)
        }
    }

    impl std::error::Error for TestError {}

    #[test]
    fn rejects_zero_max_concurrent() {
        let err = BulkheadPolicy::new(0).expect_err("zero permits should be invalid");
        assert!(matches!(err, BulkheadError::InvalidMaxConcurrent { provided: 0 }));
    }

    #[tokio::test]
    async fn test_sequential_operations_all_succeed() {
        let bulkhead = BulkheadPolicy::new(3).expect("valid bulkhead");
        let counter = Arc::new(AtomicUsize::new(0));

        // Execute 3 operations sequentially - all should succeed
        for _ in 0..3 {
            let counter_clone = counter.clone();
            let result = bulkhead
                .execute(|| {
                    let counter = counter_clone.clone();
                    async move {
                        counter.fetch_add(1, Ordering::SeqCst);
                        Ok::<_, ResilienceError<TestError>>(42)
                    }
                })
                .await;
            assert!(result.is_ok());
        }

        assert_eq!(counter.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_rejects_when_at_capacity() {
        tokio::time::pause();
        let bulkhead = BulkheadPolicy::new(2).expect("valid bulkhead");
        let notify = Arc::new(tokio::sync::Notify::new());
        let started = Arc::new(AtomicUsize::new(0));

        // Start 2 concurrent long-running operations
        let mut handles = vec![];
        for _ in 0..2 {
            let bulkhead_clone = bulkhead.clone();
            let notify_clone = notify.clone();
            let started_clone = started.clone();
            let handle = tokio::spawn(async move {
                bulkhead_clone
                    .execute(|| {
                        let notify = notify_clone.clone();
                        let started = started_clone.clone();
                        async move {
                            started.fetch_add(1, Ordering::SeqCst);
                            notify.notify_one();
                            tokio::time::sleep(Duration::from_millis(100)).await;
                            Ok::<_, ResilienceError<TestError>>(42)
                        }
                    })
                    .await
            });
            handles.push(handle);
        }

        // Wait until both tasks have acquired permits
        while started.load(Ordering::SeqCst) < 2 {
            tokio::time::advance(Duration::from_millis(1)).await;
            notify.notified().await;
        }

        // Try to execute a 3rd operation - should be rejected
        let result = bulkhead.execute(|| async { Ok::<_, ResilienceError<TestError>>(99) }).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().is_bulkhead());

        // Wait for tasks to finish
        tokio::time::advance(Duration::from_millis(100)).await;
        for handle in handles {
            let _ = handle.await;
        }
    }

    #[tokio::test]
    async fn test_releases_permits_after_completion() {
        let bulkhead = BulkheadPolicy::new(2).expect("valid bulkhead");
        let counter = Arc::new(AtomicUsize::new(0));

        // Execute 2 operations
        for _ in 0..2 {
            let counter_clone = counter.clone();
            let _ = bulkhead
                .execute(|| {
                    let counter = counter_clone.clone();
                    async move {
                        counter.fetch_add(1, Ordering::SeqCst);
                        Ok::<_, ResilienceError<TestError>>(42)
                    }
                })
                .await;
        }

        assert_eq!(counter.load(Ordering::SeqCst), 2);

        // Both should have completed, so 2 more should succeed
        counter.store(0, Ordering::SeqCst);
        for _ in 0..2 {
            let counter_clone = counter.clone();
            let result = bulkhead
                .execute(|| {
                    let counter = counter_clone.clone();
                    async move {
                        counter.fetch_add(1, Ordering::SeqCst);
                        Ok::<_, ResilienceError<TestError>>(42)
                    }
                })
                .await;
            assert!(result.is_ok());
        }

        assert_eq!(counter.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn test_unlimited_bulkhead_never_rejects() {
        tokio::time::pause();
        let bulkhead = BulkheadPolicy::unlimited();
        let mut handles = vec![];

        // Launch many concurrent operations
        for i in 0..100 {
            let bulkhead_clone = bulkhead.clone();
            let handle = tokio::spawn(async move {
                bulkhead_clone
                    .execute(|| async move {
                        tokio::time::sleep(Duration::from_millis(10)).await;
                        Ok::<_, ResilienceError<TestError>>(i)
                    })
                    .await
            });
            handles.push(handle);
        }

        // All should succeed
        tokio::time::advance(Duration::from_millis(20)).await;
        let results: Vec<_> = futures::future::join_all(handles).await;
        let successes = results.iter().filter(|r| r.as_ref().unwrap().is_ok()).count();

        assert_eq!(successes, 100, "All operations should succeed with unlimited bulkhead");
    }

    #[tokio::test]
    async fn test_concurrent_operations_up_to_limit() {
        tokio::time::pause();
        let bulkhead = BulkheadPolicy::new(5).expect("valid bulkhead");
        let concurrent_count = Arc::new(AtomicUsize::new(0));
        let max_concurrent = Arc::new(AtomicUsize::new(0));
        let mut handles = vec![];

        // Launch 10 operations (more than limit)
        for _ in 0..10 {
            let bulkhead_clone = bulkhead.clone();
            let concurrent_clone = concurrent_count.clone();
            let max_clone = max_concurrent.clone();

            let handle = tokio::spawn(async move {
                bulkhead_clone
                    .execute(|| {
                        let concurrent = concurrent_clone.clone();
                        let max = max_clone.clone();
                        async move {
                            // Track concurrent executions
                            let current = concurrent.fetch_add(1, Ordering::SeqCst) + 1;
                            max.fetch_max(current, Ordering::SeqCst);

                            tokio::time::sleep(Duration::from_millis(50)).await;

                            concurrent.fetch_sub(1, Ordering::SeqCst);
                            Ok::<_, ResilienceError<TestError>>(42)
                        }
                    })
                    .await
            });
            handles.push(handle);
        }

        tokio::time::advance(Duration::from_millis(60)).await;
        // Wait for all to complete
        let results: Vec<_> = futures::future::join_all(handles).await;

        let successes = results.iter().filter(|r| r.as_ref().unwrap().is_ok()).count();
        let rejections = results
            .iter()
            .filter(|r| r.as_ref().unwrap().as_ref().err().is_some_and(|e| e.is_bulkhead()))
            .count();

        // Should have limited concurrency to 5
        let max_observed = max_concurrent.load(Ordering::SeqCst);
        assert!(max_observed <= 5, "Should not exceed bulkhead limit of 5, got {}", max_observed);
        assert_eq!(
            successes + rejections,
            10,
            "All operations should either succeed or be rejected"
        );
    }

    #[tokio::test]
    async fn test_bulkhead_propagates_operation_errors() {
        let bulkhead = BulkheadPolicy::new(2).expect("valid bulkhead");

        let result = bulkhead
            .execute(|| async {
                Err::<(), _>(ResilienceError::Inner(TestError("operation failed".to_string())))
            })
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            ResilienceError::Inner(e) => assert_eq!(e.0, "operation failed"),
            e => panic!("Expected Inner error, got {:?}", e),
        }
    }

    #[derive(Debug, Clone, Default)]
    struct HoldService {
        hold: Arc<tokio::sync::Mutex<Option<oneshot::Receiver<()>>>>,
    }

    impl HoldService {
        fn with_block(rx: oneshot::Receiver<()>) -> Self {
            Self { hold: Arc::new(tokio::sync::Mutex::new(Some(rx))) }
        }
    }

    impl tower_service::Service<()> for HoldService {
        type Response = ();
        type Error = TestError;
        type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

        fn poll_ready(
            &mut self,
            _cx: &mut std::task::Context<'_>,
        ) -> std::task::Poll<Result<(), Self::Error>> {
            std::task::Poll::Ready(Ok(()))
        }

        fn call(&mut self, _req: ()) -> Self::Future {
            let hold = self.hold.clone();
            Box::pin(async move {
                if let Some(rx) = hold.lock().await.take() {
                    let _ = rx.await;
                }
                Ok(())
            })
        }
    }

    #[tokio::test]
    async fn service_fails_fast_when_permits_exhausted() {
        let (tx, rx) = oneshot::channel();
        let mut svc = BulkheadService::new(HoldService::with_block(rx), 1);

        // First call acquires the single permit and blocks until `tx` fires
        let first = tokio::spawn({
            let mut s = svc.clone();
            async move { s.call(()).await }
        });

        // Allow the first task to reach the await point
        tokio::task::yield_now().await;

        // Second call should fail immediately with Bulkhead error (no queuing)
        let start = tokio::time::Instant::now();
        let res = svc.call(()).await;
        assert!(res.is_err() && res.as_ref().unwrap_err().is_bulkhead());
        assert!(start.elapsed() < Duration::from_millis(20));

        // Unblock the first call and ensure it completes
        let _ = tx.send(());
        assert!(first.await.unwrap().is_ok());
    }
}
