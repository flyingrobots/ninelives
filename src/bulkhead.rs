//! Bulkhead implementation for concurrency limiting

use crate::ResilienceError;
use std::future::Future;
use std::sync::Arc;
use tokio::sync::Semaphore;

#[derive(Clone)]
pub struct BulkheadPolicy {
    semaphore: Arc<Semaphore>,
    max_concurrent: usize,
}

impl BulkheadPolicy {
    pub fn new(max_concurrent: usize) -> Self {
        Self { semaphore: Arc::new(Semaphore::new(max_concurrent)), max_concurrent }
    }

    pub fn unlimited() -> Self {
        // Semaphore::MAX_PERMITS is approximately usize::MAX / 4
        // Use a large but safe value: 1 billion concurrent operations
        Self::new(1_000_000_000)
    }

    pub async fn execute<T, E, Fut, Op>(&self, mut operation: Op) -> Result<T, ResilienceError<E>>
    where
        T: Send,
        E: std::error::Error + Send + Sync + 'static,
        Fut: Future<Output = Result<T, ResilienceError<E>>> + Send,
        Op: FnMut() -> Fut + Send,
    {
        let available = self.semaphore.available_permits();
        let in_flight = self.max_concurrent.saturating_sub(available);

        let permit = self
            .semaphore
            .try_acquire()
            .map_err(|_| ResilienceError::Bulkhead { in_flight, max: self.max_concurrent })?;

        let result = operation().await;
        drop(permit);
        result
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

    #[tokio::test]
    async fn test_allows_operations_within_limit() {
        let bulkhead = BulkheadPolicy::new(3);
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
        let bulkhead = BulkheadPolicy::new(2);
        let barrier = Arc::new(tokio::sync::Barrier::new(3)); // 2 tasks + test

        // Start 2 concurrent long-running operations
        let mut handles = vec![];
        for _ in 0..2 {
            let bulkhead_clone = bulkhead.clone();
            let barrier_clone = barrier.clone();
            let handle = tokio::spawn(async move {
                bulkhead_clone
                    .execute(|| {
                        let barrier = barrier_clone.clone();
                        async move {
                            barrier.wait().await; // Wait for all tasks to start
                            tokio::time::sleep(Duration::from_millis(100)).await;
                            Ok::<_, ResilienceError<TestError>>(42)
                        }
                    })
                    .await
            });
            handles.push(handle);
        }

        // Wait for both operations to be in-flight
        tokio::time::sleep(Duration::from_millis(10)).await;

        // Try to execute a 3rd operation - should be rejected
        let result = bulkhead.execute(|| async { Ok::<_, ResilienceError<TestError>>(99) }).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().is_bulkhead());

        // Release the barrier to let tasks complete
        barrier.wait().await;

        // Wait for tasks to finish
        for handle in handles {
            let _ = handle.await;
        }
    }

    #[tokio::test]
    async fn test_releases_permits_after_completion() {
        let bulkhead = BulkheadPolicy::new(2);
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
        let bulkhead = BulkheadPolicy::new(5);
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
            .filter(|r| r.as_ref().unwrap().as_ref().err().map_or(false, |e| e.is_bulkhead()))
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
        let bulkhead = BulkheadPolicy::new(2);

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
}
