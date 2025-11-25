//! Timeout policy for bounding async operation duration.
//!
//! Semantics
//! - Wraps an async operation and returns `ResilienceError::Timeout` when the deadline elapses.
//! - Uses `tokio::time::timeout`; on timeout the inner future is dropped (not forcibly aborted),
//!   so cancellation-unsafe work may leave partial state. Prefer cancellation-safe primitives or
//!   cooperative cancellation if that matters.
//! - Elapsed is measured from just before invoking the closure to timeout firing and can be
//!   slightly greater than the configured duration due to scheduling/timeout detection overhead.
//! - Requires a Tokio runtime.
//!
//! Invariants:
//! - Duration must be > 0 and ≤ configured maximum.
//! - Successful operations pass through untouched.
//! - Timeouts return `ResilienceError::Timeout` with elapsed ≥ configured timeout.
//!
//! Example
//! ```
//! use ninelives::{ResilienceError, TimeoutPolicy};
//! use std::time::Duration;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), ResilienceError<std::io::Error>> {
//!     let timeout = TimeoutPolicy::new(Duration::from_millis(50)).unwrap();
//!
//!     let result: Result<_, ResilienceError<std::io::Error>> = timeout
//!         .execute(|| async {
//!             tokio::time::sleep(Duration::from_millis(200)).await;
//!             Ok::<_, ResilienceError<std::io::Error>>(())
//!         })
//!         .await;
//!
//!     match result {
//!         Ok(_) => println!("done"),
//!         Err(e) if e.is_timeout() => {
//!             println!("timed out after {:?}", e.timeout_details().unwrap().1)
//!         }
//!         Err(e) => return Err(e),
//!     }
//!     Ok(())
//! }
//! ```

use crate::ResilienceError;
use futures::future::BoxFuture;
use std::future::Future;
use std::time::Duration;
use tokio::time::Instant;
use tower_service::Service;

/// Maximum allowed timeout duration (30 days) to avoid runaway timers while permitting long jobs.
/// Intended to guard accidental timeouts of `u64::MAX`; override via [`TimeoutPolicy::new_with_max`]
/// when longer horizons are required.
pub const MAX_TIMEOUT: Duration = Duration::from_secs(30 * 24 * 60 * 60);

/// Errors returned when configuring timeouts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeoutError {
    /// Duration must be greater than zero.
    ZeroDuration,
    /// Duration exceeded configured maximum.
    ExceedsMaximum {
        /// Duration requested by caller.
        requested: Duration,
        /// Maximum allowed duration for this construction.
        limit: Duration,
    },
}

impl std::fmt::Display for TimeoutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TimeoutError::ZeroDuration => write!(f, "timeout duration must be > 0"),
            TimeoutError::ExceedsMaximum { requested, limit } => write!(
                f,
                "timeout duration {:?} exceeds maximum allowed {:?}; use new_with_max to override",
                requested, limit
            ),
        }
    }
}

impl std::error::Error for TimeoutError {}

/// Policy that enforces a maximum duration on async operations.
#[derive(Debug, Clone, Copy)]
pub struct TimeoutPolicy {
    duration: Duration,
}

impl TimeoutPolicy {
    /// Creates a timeout policy with the specified duration.
    ///
    /// # Errors
    ///
    /// Returns [`TimeoutError::ZeroDuration`] if `duration` is zero.
    /// Returns [`TimeoutError::ExceedsMaximum`] if `duration` exceeds [`MAX_TIMEOUT`].
    #[must_use = "the result must be checked for validation errors"]
    pub fn new(duration: Duration) -> Result<Self, TimeoutError> {
        Self::new_with_max(duration, MAX_TIMEOUT)
    }

    /// Construct with a caller-specified maximum allowed timeout.
    pub fn new_with_max(duration: Duration, max: Duration) -> Result<Self, TimeoutError> {
        if duration.is_zero() {
            return Err(TimeoutError::ZeroDuration);
        }
        if duration > max {
            return Err(TimeoutError::ExceedsMaximum { requested: duration, limit: max });
        }
        Ok(Self { duration })
    }

    /// Returns the configured timeout duration.
    #[must_use]
    #[inline]
    pub fn duration(&self) -> Duration {
        self.duration
    }

    /// Execute an operation with a timeout.
    ///
    /// - Returns `Ok(T)` when the operation finishes before the deadline.
    /// - Returns `Err(ResilienceError::Timeout { elapsed, timeout })` when the deadline elapses.
    /// - On timeout, the inner future is dropped (Tokio does not forcibly abort); ensure the
    ///   operation is cancellation-safe if partial work matters.
    /// - `elapsed` is measured from before the operation is invoked and can exceed `timeout`
    ///   slightly due to scheduling/timeout detection overhead.
    ///
    /// # Examples
    /// ```
    /// use ninelives::{ResilienceError, TimeoutPolicy};
    /// use std::time::Duration;
    /// # tokio::runtime::Runtime::new().unwrap().block_on(async {
    /// let timeout = TimeoutPolicy::new(Duration::from_millis(20)).unwrap();
    /// let ok = timeout.execute(|| async { Ok::<_, ResilienceError<std::io::Error>>(42) }).await;
    /// assert_eq!(ok.unwrap(), 42);
    /// let timed_out = timeout.execute(|| async {
    ///     tokio::time::sleep(Duration::from_millis(100)).await;
    ///     Ok::<_, ResilienceError<std::io::Error>>(())
    /// }).await;
    /// assert!(timed_out.unwrap_err().is_timeout());
    /// # });
    /// ```
    pub async fn execute<T, E, Fut, Op>(&self, operation: Op) -> Result<T, ResilienceError<E>>
    where
        T: Send,
        E: std::error::Error + Send + Sync + 'static,
        Fut: Future<Output = Result<T, ResilienceError<E>>> + Send,
        Op: FnOnce() -> Fut + Send,
    {
        let start = Instant::now();

        match tokio::time::timeout(self.duration, operation()).await {
            Ok(result) => result,
            Err(_) => {
                let elapsed = start.elapsed();
                Err(ResilienceError::Timeout { elapsed, timeout: self.duration })
            }
        }
    }
}

use crate::telemetry::{emit_best_effort, NullSink, PolicyEvent, RequestOutcome, TimeoutEvent};

/// Tower-native timeout layer with optional telemetry.
#[derive(Clone)]
pub struct TimeoutLayer<Sink = NullSink> {
    duration: Duration,
    sink: Sink,
}

impl TimeoutLayer<NullSink> {
    /// Build a timeout layer with the provided duration and no telemetry.
    pub fn new(duration: Duration) -> Result<Self, TimeoutError> {
        TimeoutPolicy::new(duration)
            .map(|p| TimeoutLayer { duration: p.duration, sink: NullSink })
    }
}

impl<Sink> TimeoutLayer<Sink>
where
    Sink: Clone,
{
    /// Attach a telemetry sink to this timeout layer.
    pub fn with_sink<NewSink>(self, sink: NewSink) -> TimeoutLayer<NewSink>
    where
        NewSink: Clone,
    {
        TimeoutLayer {
            duration: self.duration,
            sink,
        }
    }
}

/// Service produced by [`TimeoutLayer`]; wraps an inner service with a timeout.
#[derive(Clone)]
pub struct TimeoutService<S, Sink = NullSink> {
    inner: S,
    duration: Duration,
    sink: Sink,
}

impl<S, Sink> TimeoutService<S, Sink> {
    fn new(inner: S, duration: Duration, sink: Sink) -> Self {
        Self { inner, duration, sink }
    }
}

impl<S, Request, Sink> Service<Request> for TimeoutService<S, Sink>
where
    S: Service<Request>,
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
        let duration = self.duration;
        let fut = self.inner.call(req);
        let sink = self.sink.clone();

        Box::pin(async move {
            let start = Instant::now();
            match tokio::time::timeout(duration, fut).await {
                Ok(Ok(r)) => {
                    // Emit success event
                    let elapsed = start.elapsed();
                    emit_best_effort(
                        sink.clone(),
                        PolicyEvent::Request(RequestOutcome::Success { duration: elapsed }),
                    )
                    .await;
                    Ok(r)
                }
                Ok(Err(e)) => {
                    // Emit failure event
                    let elapsed = start.elapsed();
                    emit_best_effort(
                        sink.clone(),
                        PolicyEvent::Request(RequestOutcome::Failure { duration: elapsed }),
                    )
                    .await;
                    Err(ResilienceError::Inner(e))
                }
                Err(_) => {
                    let elapsed = start.elapsed();
                    // Emit timeout event
                    emit_best_effort(
                        sink.clone(),
                        PolicyEvent::Timeout(TimeoutEvent::Occurred { timeout: duration }),
                    )
                    .await;
                    Err(ResilienceError::Timeout { elapsed, timeout: duration })
                }
            }
        })
    }
}

impl<S, Sink> tower_layer::Layer<S> for TimeoutLayer<Sink>
where
    Sink: Clone,
{
    type Service = TimeoutService<S, Sink>;
    fn layer(&self, service: S) -> Self::Service {
        TimeoutService::new(service, self.duration, self.sink.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct TestError(String);

    impl std::fmt::Display for TestError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "TestError: {}", self.0)
        }
    }

    impl std::error::Error for TestError {}

    #[tokio::test]
    async fn test_completes_before_timeout() {
        let timeout = TimeoutPolicy::new(Duration::from_millis(100)).unwrap();
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let result = timeout
            .execute(|| {
                let counter = counter_clone.clone();
                async move {
                    counter.fetch_add(1, Ordering::SeqCst);
                    tokio::time::sleep(Duration::from_millis(10)).await;
                    Ok::<_, ResilienceError<TestError>>(42)
                }
            })
            .await;

        assert_eq!(counter.load(Ordering::SeqCst), 1);
        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn test_times_out_long_operation() {
        tokio::time::pause();
        let timeout = TimeoutPolicy::new(Duration::from_millis(50)).unwrap();
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let fut = timeout.execute(|| {
            let counter = counter_clone.clone();
            async move {
                counter.fetch_add(1, Ordering::SeqCst);
                tokio::time::sleep(Duration::from_millis(500)).await;
                Ok::<_, ResilienceError<TestError>>(42)
            }
        });

        tokio::pin!(fut);
        tokio::time::advance(Duration::from_millis(51)).await;
        let result = fut.await;

        assert!(result.is_err());
        assert!(result.unwrap_err().is_timeout());
        assert_eq!(counter.load(Ordering::SeqCst), 1, "Should have started execution");
    }

    #[tokio::test]
    async fn test_propagates_operation_errors() {
        let timeout = TimeoutPolicy::new(Duration::from_secs(1)).unwrap();

        let result = timeout
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
    async fn test_very_long_timeout_doesnt_interfere() {
        let timeout = TimeoutPolicy::new(Duration::from_secs(3600)).unwrap(); // 1 hour

        let result = timeout
            .execute(|| async {
                tokio::time::sleep(Duration::from_millis(10)).await;
                Ok::<_, ResilienceError<TestError>>(99)
            })
            .await;

        assert_eq!(result.unwrap(), 99);
    }

    #[tokio::test]
    async fn test_timeout_error_includes_durations() {
        let timeout_duration = Duration::from_millis(50);
        let timeout = TimeoutPolicy::new(timeout_duration).unwrap();

        let result = timeout
            .execute(|| async {
                tokio::time::sleep(Duration::from_millis(200)).await;
                Ok::<(), ResilienceError<TestError>>(())
            })
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            ResilienceError::Timeout { elapsed, timeout } => {
                assert_eq!(timeout, timeout_duration);
                assert!(elapsed >= timeout_duration, "Elapsed time should be at least the timeout");
            }
            e => panic!("Expected Timeout error, got {:?}", e),
        }
    }

    #[tokio::test]
    async fn test_instant_operation() {
        let timeout = TimeoutPolicy::new(Duration::from_millis(100)).unwrap();

        let result = timeout.execute(|| async { Ok::<_, ResilienceError<TestError>>(42) }).await;

        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn rejects_zero_duration() {
        let err = TimeoutPolicy::new(Duration::ZERO).unwrap_err();
        assert!(matches!(err, TimeoutError::ZeroDuration));
    }

    #[test]
    fn rejects_excessive_duration() {
        let too_big = MAX_TIMEOUT + Duration::from_secs(1);
        let err = TimeoutPolicy::new(too_big).unwrap_err();
        assert!(
            matches!(err, TimeoutError::ExceedsMaximum { requested, limit } if requested == too_big && limit == MAX_TIMEOUT)
        );
    }

    #[test]
    fn accepts_max_timeout() {
        let policy = TimeoutPolicy::new(MAX_TIMEOUT).expect("should accept max boundary");
        assert_eq!(policy.duration(), MAX_TIMEOUT);
    }

    #[test]
    fn new_with_max_respects_custom_boundaries() {
        let custom_max = Duration::from_secs(5);
        let ok = TimeoutPolicy::new_with_max(Duration::from_secs(5), custom_max).unwrap();
        assert_eq!(ok.duration(), custom_max);

        let err = TimeoutPolicy::new_with_max(Duration::from_secs(6), custom_max).unwrap_err();
        assert!(
            matches!(err, TimeoutError::ExceedsMaximum { requested, limit } if requested == Duration::from_secs(6) && limit == custom_max)
        );
    }
}
