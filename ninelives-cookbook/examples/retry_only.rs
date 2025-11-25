//! Minimal, focused retry example with backoff, jitter, and a `should_retry` predicate.

use ninelives::prelude::*;
use std::{
    fmt,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};
use tower::{Service, ServiceBuilder, ServiceExt};

#[derive(Debug, Clone, PartialEq, Eq)]
enum MyError {
    Retryable(&'static str),
    Fatal(&'static str),
}

impl fmt::Display for MyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MyError::Retryable(msg) => write!(f, "retryable: {}", msg),
            MyError::Fatal(msg) => write!(f, "fatal: {}", msg),
        }
    }
}

impl std::error::Error for MyError {}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Nine Lives: Retry-Only Example ===\n");

    // Policy: 4 total attempts, exponential backoff starting at 50ms, full jitter, retry only on Retryable.
    let retry_policy = RetryPolicy::builder()
        .max_attempts(4)
        .backoff(Backoff::exponential(Duration::from_millis(50)))
        .with_jitter(Jitter::full())
        .should_retry(|err: &MyError| matches!(err, MyError::Retryable(_)))
        .build()?;

    // Attach a MemorySink so we can print telemetry events at the end.
    let sink = MemorySink::new();
    let retry_layer = retry_policy.into_layer().with_sink(sink.clone());

    // A flaky service: first two attempts are retryable failures, third succeeds unless we trigger a fatal path.
    let attempt = Arc::new(AtomicUsize::new(0));
    let attempt_clone = attempt.clone();
    let mut svc = ServiceBuilder::new().layer(retry_layer).service_fn(move |req: &'static str| {
        let n = attempt_clone.fetch_add(1, Ordering::SeqCst);
        async move {
            if req == "fatal" {
                return Err(MyError::Fatal("do not retry"));
            }

            match n {
                0 | 1 => Err(MyError::Retryable("transient upstream")),
                _ => Ok::<_, MyError>(format!("ok on attempt {}", n + 1)),
            }
        }
    });

    println!("Calling flaky service (should succeed after retries)...");
    let ok = svc.ready().await?.call("happy").await?;
    println!("✓ Result: {}", ok);

    println!("\nCalling fatal path (should NOT retry)...");
    let err = svc.ready().await?.call("fatal").await.unwrap_err();
    println!("✗ Fatal error returned immediately: {}", err);

    println!("\nTelemetry events (MemorySink):");
    for event in sink.events() {
        println!("  - {}", event);
    }

    Ok(())
}
