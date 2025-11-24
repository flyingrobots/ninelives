use ninelives::{
    Backoff, Jitter, ResilienceError, ResilienceStack, ResilienceStackBuilder, RetryPolicy,
};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

#[tokio::test]
async fn retry_retries_inner_errors_then_succeeds() {
    let retry = RetryPolicy::builder()
        .max_attempts(3)
        .backoff(Backoff::constant(Duration::from_millis(0)))
        .with_jitter(Jitter::None)
        .build()
        .unwrap();

    let stack: ResilienceStack<TestError> =
        ResilienceStackBuilder::new().retry(retry).build().unwrap();

    let attempts = Arc::new(AtomicUsize::new(0));
    let attempts_clone = attempts.clone();

    let result = stack
        .execute(move || {
            let attempts = attempts_clone.clone();
            async move {
                let n = attempts.fetch_add(1, Ordering::SeqCst);
                if n < 2 {
                    Err(ResilienceError::Inner(TestError))
                } else {
                    Ok::<_, ResilienceError<TestError>>(())
                }
            }
        })
        .await;

    assert!(result.is_ok());
    assert_eq!(attempts.load(Ordering::SeqCst), 3);
}

#[tokio::test]
async fn bulkhead_rejects_when_full() {
    let stack: ResilienceStack<TestError> = ResilienceStackBuilder::new()
        .bulkhead(1)
        .unwrap()
        .timeout(Duration::from_secs(1))
        .unwrap()
        .build()
        .unwrap();

    let holding = stack.clone();
    let holder = tokio::spawn(async move {
        holding
            .execute(|| async {
                tokio::time::sleep(Duration::from_millis(200)).await;
                Ok::<_, ResilienceError<TestError>>(())
            })
            .await
    });

    tokio::time::sleep(Duration::from_millis(20)).await;

    let rejected = stack.execute(|| async { Ok::<_, ResilienceError<TestError>>(()) }).await;

    assert!(matches!(rejected, Err(e) if e.is_bulkhead()));
    let _ = holder.await.unwrap();
}

#[tokio::test]
async fn timeout_triggers_on_slow_operation() {
    let stack: ResilienceStack<TestError> = ResilienceStackBuilder::new()
        .timeout(Duration::from_millis(50))
        .unwrap()
        .bulkhead(2)
        .unwrap()
        .build()
        .unwrap();

    let result = stack
        .execute(|| async {
            tokio::time::sleep(Duration::from_millis(200)).await;
            Ok::<_, ResilienceError<TestError>>(())
        })
        .await;

    assert!(matches!(result, Err(e) if e.is_timeout()));
}

#[tokio::test]
async fn circuit_breaker_opens_after_failure() {
    let retry = RetryPolicy::builder()
        .max_attempts(1)
        .backoff(Backoff::constant(Duration::from_millis(0)))
        .with_jitter(Jitter::None)
        .build()
        .unwrap();

    let stack: ResilienceStack<TestError> = ResilienceStackBuilder::new()
        .circuit_breaker(1, Duration::from_secs(30))
        .unwrap()
        .retry(retry)
        .build()
        .unwrap();

    let _ = stack.execute(|| async { Err::<(), _>(ResilienceError::Inner(TestError)) }).await;

    let second = stack.execute(|| async { Ok::<_, ResilienceError<TestError>>(()) }).await;

    assert!(matches!(second, Err(e) if e.is_circuit_open()));
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TestError;

impl std::fmt::Display for TestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "test error")
    }
}

impl std::error::Error for TestError {}
