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
use std::future::Future;
use std::time::{Duration, Instant};

/// Maximum allowed timeout duration (30 days) to avoid runaway timers while permitting long jobs.
/// Intended to guard accidental timeouts of `u64::MAX`; override via [`TimeoutPolicy::new_with_max`]
/// when longer horizons are required.
pub const MAX_TIMEOUT: Duration = Duration::from_secs(30 * 24 * 60 * 60);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeoutError {
    ZeroDuration,
    ExceedsMaximum(Duration),
}

impl std::fmt::Display for TimeoutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TimeoutError::ZeroDuration => write!(f, "timeout duration must be > 0"),
            TimeoutError::ExceedsMaximum(d) => {
                write!(
                    f,
                    "timeout duration {:?} exceeds maximum allowed {:?}; use new_with_max to override",
                    d, MAX_TIMEOUT
                )
            }
        }
    }
}

impl std::error::Error for TimeoutError {}

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
            return Err(TimeoutError::ExceedsMaximum(duration));
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
                assert!(
                    elapsed < timeout_duration * 2,
                    "Elapsed should not be excessively larger than timeout"
                );
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
        assert!(matches!(err, TimeoutError::ExceedsMaximum(_)));
    }

    #[test]
    fn accepts_max_timeout() {
        let policy = TimeoutPolicy::new(MAX_TIMEOUT).expect("should accept max boundary");
        assert_eq!(policy.duration(), MAX_TIMEOUT);
    }
}
