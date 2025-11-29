//! Bulkhead implementation for concurrency limiting.
//!
//! A bulkhead caps concurrent operations to protect downstream services and bound resource usage.
//! This implementation is non-blocking: when no permits are available it rejects immediately
//! (`ResilienceError::Bulkhead`) rather than queuing. Permits are released when the wrapped
//! operation finishes.
//!
//! Example
//! ```rust
//! use ninelives::{BulkheadPolicy, ResilienceError};
//! use std::sync::atomic::{AtomicUsize, Ordering};
//!
//! # tokio::runtime::Runtime::new().unwrap().block_on(async {
//! let bulkhead = BulkheadPolicy::new(1).expect("valid bulkhead");
//! let counter = AtomicUsize::new(0);
//! let (started_tx, started_rx) = tokio::sync::oneshot::channel();
//! let (release_tx, release_rx) = tokio::sync::oneshot::channel();
//! let holder = tokio::spawn({
//!     let bh = bulkhead.clone();
//!     async move {
//!         let _ = bh.execute(|| async {
//!             counter.fetch_add(1, Ordering::SeqCst);
//!             let _ = started_tx.send(());
//!             let _ = release_rx.await;
//!             Ok::<_, ResilienceError<std::io::Error>>(())
//!         }).await;
//!     }
//! });
//! started_rx.await.unwrap();
//! let rejected = bulkhead.execute(|| async { Ok::<_, ResilienceError<std::io::Error>>(()) }).await;
//! assert!(rejected.unwrap_err().is_bulkhead());
//! let _ = release_tx.send(());
//! let _ = holder.await;
//! # });
//! ```

use crate::{adaptive::Adaptive, ResilienceError};
use futures::future::BoxFuture;
use std::future::Future;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use tokio::sync::Semaphore;
use tower_layer::Layer;
use tower_service::Service;

#[derive(Debug, Clone)]
/// Concurrency-limiting bulkhead. Clones share the same underlying semaphore via `Arc`, so they
/// observe and affect the same in-flight count. `max_concurrent` is the configured permit ceiling.
pub struct BulkheadPolicy {
    semaphore: Arc<Semaphore>,
    /// Mirrors the initial semaphore capacity; used only for reporting and adaptation.
    max_concurrent: Adaptive<usize>,
    capacity: Arc<AtomicUsize>,
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

        Ok(Self {
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
            max_concurrent: Adaptive::new(max_concurrent),
            capacity: Arc::new(AtomicUsize::new(max_concurrent)),
        })
    }

    /// Construct an effectively unlimited bulkhead using `UNLIMITED_PERMITS` (derived from
    /// `Semaphore::MAX_PERMITS`). Operations still reject immediately if all permits are in use.
    pub fn unlimited() -> Self {
        // Safe: constant respects semaphore maximum.
        Self::new(UNLIMITED_PERMITS).unwrap()
    }

    /// Maximum configured concurrent permits.
    pub fn max_concurrent(&self) -> usize {
        *self.max_concurrent.get()
    }

    /// Returns a handle to the adaptive configuration for max concurrent permits.
    pub fn adaptive_max_concurrent(&self) -> Adaptive<usize> {
        self.max_concurrent.clone()
    }

    /// Best-effort current available permits (may be stale due to races).
    pub fn available_permits(&self) -> usize {
        self.semaphore.available_permits()
    }

    /// Execute an operation with bulkhead protection. Non-blocking: if no permits are available
    /// the call is rejected immediately with `ResilienceError::Bulkhead`. Permits are released when
    /// the operation future completes. The reported `in_flight` is a best-effort snapshot and may
    /// be stale due to races.
    pub async fn execute<T, E, Fut, Op>(&self, operation: Op) -> Result<T, ResilienceError<E>>
    where
        T: Send,
        E: std::error::Error + Send + Sync + 'static,
        Fut: Future<Output = Result<T, ResilienceError<E>>> + Send,
        Op: FnOnce() -> Fut + Send,
    {
        self.sync_capacity();
        let _permit = match self.semaphore.try_acquire() {
            Ok(p) => p,
            Err(tokio::sync::TryAcquireError::NoPermits) => {
                let desired = *self.max_concurrent.get();
                let available = self.semaphore.available_permits(); // best-effort snapshot
                let in_flight = desired.saturating_sub(available.min(desired));
                return Err(ResilienceError::Bulkhead { in_flight, max: desired });
            }
            Err(tokio::sync::TryAcquireError::Closed) => {
                let max = *self.max_concurrent.get();
                return Err(ResilienceError::Bulkhead { in_flight: max, max });
            }
        };

        // permit released on scope exit
        operation().await
    }

    fn sync_capacity(&self) {
        let desired = (*self.max_concurrent.get()).max(1);
        loop {
            let current = self.capacity.load(Ordering::Acquire);
            let target = desired; // enforce minimum of 1
            if target <= current {
                break;
            }
            // CAS to avoid double-adding permits
            if self
                .capacity
                .compare_exchange_weak(current, target, Ordering::AcqRel, Ordering::Relaxed)
                .is_ok()
            {
                self.semaphore.add_permits(target - current);
                break;
            }
        }
    }
}

use crate::telemetry::{emit_best_effort, BulkheadEvent, NullSink, PolicyEvent, RequestOutcome};
use std::time::Instant as StdInstant;

/// Tower-native bulkhead layer with optional telemetry.
#[derive(Clone)]
pub struct BulkheadLayer<Sink = NullSink> {
    max_concurrent: Adaptive<usize>,
    sink: Sink,
}

impl BulkheadLayer<NullSink> {
    /// Create a bulkhead layer with no telemetry; returns error if `max_concurrent` is zero.
    pub fn new(max_concurrent: usize) -> Result<Self, BulkheadError> {
        BulkheadPolicy::new(max_concurrent)?;
        Ok(Self { max_concurrent: Adaptive::new(max_concurrent), sink: NullSink })
    }
}

impl<Sink> BulkheadLayer<Sink>
where
    Sink: Clone,
{
    /// Attach a telemetry sink to this bulkhead layer.
    pub fn with_sink<NewSink>(self, sink: NewSink) -> BulkheadLayer<NewSink>
    where
        NewSink: Clone,
    {
        BulkheadLayer { max_concurrent: self.max_concurrent.clone(), sink }
    }
}

/// Service produced by [`BulkheadLayer`]; enforces permit limits.
#[derive(Clone)]
pub struct BulkheadService<S, Sink = NullSink> {
    semaphore: Arc<Semaphore>,
    max_concurrent: Adaptive<usize>,
    capacity: Arc<AtomicUsize>,
    inner: S,
    sink: Sink,
}

impl<S, Sink> BulkheadService<S, Sink> {
    fn new(inner: S, max_concurrent: Adaptive<usize>, sink: Sink) -> Self {
        let initial = (*max_concurrent.get()).max(1);
        Self {
            semaphore: Arc::new(Semaphore::new(initial)),
            max_concurrent,
            capacity: Arc::new(AtomicUsize::new(initial)),
            inner,
            sink,
        }
    }
}

impl<S, Request, Sink> Service<Request> for BulkheadService<S, Sink>
where
    S: Service<Request> + Clone + Send + 'static,
    S::Future: Send + 'static,
    Request: Send + 'static,
    S::Error: std::error::Error + Send + Sync + 'static,
    S::Response: Send + 'static,
    Sink: tower::Service<PolicyEvent, Response = ()> + Clone + Send + 'static,
    Sink::Error: std::error::Error + Send + 'static,
    Sink::Future: Send + 'static,
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
        let start = StdInstant::now();
        let semaphore = self.semaphore.clone();
        let mut inner = self.inner.clone();
        let max = self.max_concurrent.clone();
        let capacity = self.capacity.clone();
        let sink = self.sink.clone();

        Box::pin(async move {
            // Sync capacity upward if desired increased
            let desired = *max.get();
            let current_cap = capacity.load(Ordering::Acquire);
            if desired > current_cap {
                semaphore.add_permits(desired - current_cap);
                capacity.store(desired, Ordering::Release);
            }

            // Check available permits before acquiring
            let available_before = semaphore.available_permits();

            // Try to acquire permit
            let permit = match semaphore.clone().try_acquire_owned() {
                Ok(p) => {
                    let max_cfg = *max.get();
                    let active_count = max_cfg.saturating_sub(available_before.min(max_cfg)) + 1;
                    // Emit acquired event
                    emit_best_effort(
                        sink.clone(),
                        PolicyEvent::Bulkhead(BulkheadEvent::Acquired {
                            active_count,
                            max_concurrency: max_cfg,
                        }),
                    )
                    .await;
                    p
                }
                Err(_) => {
                    let max_cfg = *max.get();
                    let active_count = max_cfg;
                    // Emit rejected event
                    emit_best_effort(
                        sink.clone(),
                        PolicyEvent::Bulkhead(BulkheadEvent::Rejected {
                            active_count,
                            max_concurrency: max_cfg,
                        }),
                    )
                    .await;
                    return Err(ResilienceError::Bulkhead {
                        in_flight: active_count,
                        max: max_cfg,
                    });
                }
            };

            // Execute request
            let result = inner.call(req).await;

            // Release permit
            drop(permit);

            // Emit outcome event
            let duration = start.elapsed();
            match &result {
                Ok(_) => {
                    emit_best_effort(
                        sink.clone(),
                        PolicyEvent::Request(RequestOutcome::Success { duration }),
                    )
                    .await;
                }
                Err(_) => {
                    emit_best_effort(
                        sink.clone(),
                        PolicyEvent::Request(RequestOutcome::Failure { duration }),
                    )
                    .await;
                }
            }

            result.map_err(ResilienceError::Inner)
        })
    }
}

impl<S, Sink> Layer<S> for BulkheadLayer<Sink>
where
    Sink: Clone,
{
    type Service = BulkheadService<S, Sink>;
    fn layer(&self, service: S) -> Self::Service {
        BulkheadService::new(service, self.max_concurrent.clone(), self.sink.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::time::Duration;

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
            notify.notified().await;
        }

        // Try to execute a 3rd operation - should be rejected
        let result = bulkhead.execute(|| async { Ok::<_, ResilienceError<TestError>>(99) }).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().is_bulkhead());

        // Wait for tasks to finish
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
        let results: Vec<_> = futures::future::join_all(handles).await;
        let successes = results.iter().filter(|r| r.as_ref().unwrap().is_ok()).count();

        assert_eq!(successes, 100, "All operations should succeed with unlimited bulkhead");
    }

    #[tokio::test]
    async fn test_concurrent_operations_up_to_limit() {
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

    #[tokio::test]
    async fn test_permit_released_after_error() {
        let bulkhead = BulkheadPolicy::new(1).unwrap();
        let bh = bulkhead.clone();
        let first = bh.execute(|| async move {
            Err::<(), ResilienceError<TestError>>(ResilienceError::BulkheadClosed)
        });
        assert!(first.await.is_err());

        // Permit should be free for next call
        let res = bulkhead.execute(|| async move { Ok::<_, ResilienceError<TestError>>(()) }).await;
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn bulkhead_layer_rejects_when_full() {
        use crate::telemetry::NullSink;
        use std::pin::Pin;
        use std::task::{Context, Poll};

        struct BlockingSvc(
            std::sync::Arc<std::sync::Mutex<Option<tokio::sync::oneshot::Receiver<()>>>>,
        );

        impl Clone for BlockingSvc {
            fn clone(&self) -> Self {
                Self(self.0.clone())
            }
        }

        impl tower_service::Service<&'static str> for BlockingSvc {
            type Response = &'static str;
            type Error = std::io::Error;
            type Future =
                Pin<Box<dyn futures::Future<Output = Result<Self::Response, Self::Error>> + Send>>;
            fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
                Poll::Ready(Ok(()))
            }
            fn call(&mut self, req: &'static str) -> Self::Future {
                let rx = self.0.lock().unwrap().take().expect("receiver");
                Box::pin(async move {
                    let _ = rx.await;
                    Ok(req)
                })
            }
        }

        let (tx, rx) = tokio::sync::oneshot::channel();
        let layer = BulkheadLayer::new(1).unwrap().with_sink(NullSink);
        let svc = layer.layer(BlockingSvc(std::sync::Arc::new(std::sync::Mutex::new(Some(rx)))));

        // First call holds permit
        let mut svc1 = svc.clone();
        let hold = tokio::spawn(async move { svc1.call("held").await });

        // Yield to let the spawned task run and acquire the permit
        tokio::task::yield_now().await;

        // Second call should be rejected immediately
        let mut svc2 = svc.clone();
        let err = svc2.call("rejected").await.unwrap_err();
        assert!(matches!(err, ResilienceError::Bulkhead { .. }));

        let _ = tx.send(());
        let _ = hold.await;
    }
}
