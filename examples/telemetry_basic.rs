//! Basic telemetry integration example.
//!
//! Demonstrates how to attach telemetry sinks to policies and observe events.

use ninelives::prelude::*;
use std::time::Duration;
use tower::{Service, ServiceBuilder, ServiceExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing for LogSink
    tracing_subscriber::fmt::init();

    println!("=== Basic Telemetry Example ===\n");

    // Example 1: Retry with MemorySink
    println!("Example 1: Retry with MemorySink");
    let memory_sink = MemorySink::new();

    let retry_policy = RetryPolicy::<std::io::Error>::builder()
        .max_attempts(3)
        .backoff(Backoff::constant(Duration::from_millis(100)))
        .build()?;

    let retry_layer = retry_policy.into_layer().with_sink(memory_sink.clone());

    let attempt = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let attempt_clone = attempt.clone();
    let mut svc = ServiceBuilder::new()
        .layer(retry_layer)
        .service_fn(move |_req: &str| {
            let count = attempt_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            async move {
                if count < 1 {
                    Err(std::io::Error::new(std::io::ErrorKind::Other, "temporary failure"))
                } else {
                    Ok::<_, std::io::Error>("success!")
                }
            }
        });

    let result = svc.ready().await?.call("request").await;
    println!("Result: {:?}", result);

    println!("\nCaptured telemetry events:");
    for event in memory_sink.events() {
        println!("  - {}", event);
    }

    // Example 2: Circuit breaker with LogSink
    println!("\n\nExample 2: Circuit breaker with LogSink");

    let circuit_config = CircuitBreakerConfig::new(
        2,                             // failure threshold
        Duration::from_secs(5),        // recovery timeout
        1,                             // half-open max calls
    )?;

    let circuit_layer = CircuitBreakerLayer::new(circuit_config)?.with_sink(LogSink);

    let fail_count = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let fail_count_clone = fail_count.clone();
    let mut svc = ServiceBuilder::new()
        .layer(circuit_layer)
        .service_fn(move |_req: &str| {
            let count = fail_count_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            async move {
                if count < 2 {
                    Err(std::io::Error::new(std::io::ErrorKind::Other, "failing"))
                } else {
                    Ok::<_, std::io::Error>("recovered")
                }
            }
        });

    // First two calls will fail
    for i in 1..=2 {
        println!("Call {}", i);
        let _ = svc.ready().await?.call("request").await;
    }

    // Circuit should now be open
    println!("Call 3 (circuit should be open)");
    let result = svc.ready().await?.call("request").await;
    println!("Result: {:?}", result);

    // Example 3: Timeout with StreamingSink
    println!("\n\nExample 3: Timeout with StreamingSink");

    let streaming_sink = StreamingSink::new(100);
    let mut receiver = streaming_sink.subscribe();

    let timeout_layer = TimeoutLayer::new(Duration::from_millis(50))?.with_sink(streaming_sink);

    let mut svc = ServiceBuilder::new()
        .layer(timeout_layer)
        .service_fn(|req: &str| {
            let slow = req == "slow";
            async move {
                if slow {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
                Ok::<_, std::io::Error>("done")
            }
        });

    // Spawn a task to print events as they arrive
    let event_printer = tokio::spawn(async move {
        while let Ok(event) = receiver.recv().await {
            println!("  [event] {}", event);
        }
    });

    println!("Fast request:");
    let _ = svc.ready().await?.call("fast").await;
    tokio::time::sleep(Duration::from_millis(10)).await;

    println!("\nSlow request (will timeout):");
    let _ = svc.ready().await?.call("slow").await;
    tokio::time::sleep(Duration::from_millis(10)).await;

    // Clean shutdown
    drop(svc);
    event_printer.abort();

    Ok(())
}
