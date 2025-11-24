//! Timeout policy implementation

use crate::ResilienceError;
use std::future::Future;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct TimeoutPolicy {
    duration: Duration,
}

impl TimeoutPolicy {
    /// Create a timeout policy. Panics if duration is zero or `Duration::MAX`.
    pub fn new(duration: Duration) -> Self {
        assert!(
            duration > Duration::ZERO && duration < Duration::MAX,
            "timeout duration must be non-zero and finite",
        );
        Self { duration }
    }

    /// Inspect the configured timeout duration.
    pub fn duration(&self) -> Duration {
        self.duration
    }

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
        let timeout = TimeoutPolicy::new(Duration::from_millis(100));
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

        assert_eq!(result.unwrap(), 42);
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_times_out_long_operation() {
        let timeout = TimeoutPolicy::new(Duration::from_millis(50));
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let result = timeout
            .execute(|| {
                let counter = counter_clone.clone();
                async move {
                    counter.fetch_add(1, Ordering::SeqCst);
                    tokio::time::sleep(Duration::from_millis(200)).await;
                    Ok::<_, ResilienceError<TestError>>(42)
                }
            })
            .await;

        assert!(result.is_err());
        assert!(result.unwrap_err().is_timeout());
        assert_eq!(counter.load(Ordering::SeqCst), 1, "Should have started execution");
    }

    #[tokio::test]
    async fn test_propagates_operation_errors() {
        let timeout = TimeoutPolicy::new(Duration::from_secs(1));

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
        let timeout = TimeoutPolicy::new(Duration::from_secs(3600)); // 1 hour

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
        let timeout = TimeoutPolicy::new(timeout_duration);

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
        let timeout = TimeoutPolicy::new(Duration::from_millis(100));

        let result = timeout.execute(|| async { Ok::<_, ResilienceError<TestError>>(42) }).await;

        assert_eq!(result.unwrap(), 42);
    }
}
