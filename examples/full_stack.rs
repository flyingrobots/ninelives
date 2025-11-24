//! Full stack example: retry + breaker + bulkhead + timeout.
use ninelives::prelude::*;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), ResilienceError<std::io::Error>> {
    let attempts = Arc::new(AtomicUsize::new(0));

    let stack: ResilienceStack<std::io::Error> = ResilienceStack::new()
        .timeout(Duration::from_secs(2))
        .expect("valid timeout")
        .bulkhead(32)
        .expect("valid bulkhead")
        .circuit_breaker(5, Duration::from_secs(30))
        .expect("valid breaker")
        .retry(
            RetryPolicy::builder()
                .max_attempts(4)
                .backoff(
                    Backoff::exponential(Duration::from_millis(100))
                        .with_max(Duration::from_secs(1))
                        .expect("valid cap"),
                )
                .with_jitter(Jitter::equal())
                .build()
                .expect("valid retry policy"),
        )
        .build()
        .expect("valid stack");

    let result = stack
        .execute(|| {
            let attempts = attempts.clone();
            async move {
                let n = attempts.fetch_add(1, Ordering::SeqCst);
                if n < 2 {
                    Err(ResilienceError::Inner(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "transient",
                    )))
                } else {
                    Ok::<_, ResilienceError<std::io::Error>>("recovered")
                }
            }
        })
        .await?;

    println!("stack result: {result}");
    Ok(())
}
