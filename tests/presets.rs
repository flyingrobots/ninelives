#[cfg(test)]
#[cfg(feature = "control")] // Presets use telemetry and other control-plane related features
mod tests {
    use ninelives::presets;
    use ninelives::telemetry::{MemorySink, PolicyEvent, RetryEvent}; // Removed PolicyEventKind
    use tower::service_fn;
    use tower::ServiceExt; // Still used by .oneshot()
    use std::sync::{Arc, atomic::{AtomicUsize, Ordering}};

    // Custom Error type that implements Clone
    #[derive(Debug, Clone, PartialEq, Eq)]
    struct TestError(String);

    impl std::fmt::Display for TestError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.0)
        }
    }

    impl std::error::Error for TestError {}

    // Helper service that always fails after a certain number of calls
    struct FailingService {
        fail_count: Arc<AtomicUsize>,
        remaining_fails: usize,
    }

    impl FailingService {
        fn new(remaining_fails: usize) -> Self {
            FailingService {
                fail_count: Arc::new(AtomicUsize::new(0)),
                remaining_fails,
            }
        }
        // new_arc removed
        async fn call(&self, req: &'static str) -> Result<String, TestError> { // Changed to TestError
            let current_fails = self.fail_count.fetch_add(1, Ordering::SeqCst);
            if current_fails < self.remaining_fails {
                Err(TestError(format!("simulated error after {} calls for req: {}", current_fails, req))) // Changed to TestError
            } else {
                Ok(format!("success for req: {}", req))
            }
        }
    }


    #[tokio::test]
    async fn web_service_preset_handles_failures() {
        let sink = MemorySink::with_capacity(100);
        let failing_svc = Arc::new(FailingService::new(2)); // Fails 2 times
        
        let svc = presets::web_service(
            service_fn(move |req: &'static str| {
                let failing_svc = failing_svc.clone();
                async move {
                    failing_svc.call(req).await
                }
            }),
            sink.clone(),
        );

        // Expect 3 retries (total 4 calls) to succeed on the 4th call
        let result = svc.oneshot("test_web_service").await;
        
        assert!(result.is_ok(), "Web service preset should eventually succeed");
        assert_eq!(result.unwrap(), "success for req: test_web_service");

        // Verify telemetry captured retry events
        let events = sink.events(); // Removed .await
        let retry_events: Vec<&PolicyEvent> = events.iter()
            .filter(|e| matches!(e, PolicyEvent::Retry(RetryEvent::Attempt {..}))) // Corrected matching
            .collect();
        // Expect (max_attempts - 1) retry attempts to be logged
        assert_eq!(retry_events.len(), 2, "Expected 2 retry attempts"); 
    }

    #[tokio::test]
    async fn database_preset_does_not_retry() {
        let sink = MemorySink::with_capacity(100);
        let failing_svc = Arc::new(FailingService::new(1)); // Fails once
        
        let svc = presets::database_client(
            service_fn(move |req: &'static str| {
                let failing_svc = failing_svc.clone();
                async move {
                    failing_svc.call(req).await
                }
            }),
            sink.clone(),
        );

        // Should fail immediately without retry
        let result = svc.oneshot("test_db_service").await;
        assert!(result.is_err(), "Database preset should fail immediately");
        assert!(result.unwrap_err().to_string().contains("simulated error"), "Expected simulated error");

        // Verify NO retry events in telemetry
        let events = sink.events(); // Removed .await
        let retry_events: Vec<&PolicyEvent> = events.iter()
            .filter(|e| matches!(e, PolicyEvent::Retry(RetryEvent::Attempt {..}))) // Corrected matching
            .collect();
        assert_eq!(retry_events.len(), 0, "Expected no retry attempts for database preset");
    }
}