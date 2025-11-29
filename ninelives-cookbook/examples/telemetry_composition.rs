//! Demonstrates composing telemetry sinks with `MulticastSink` and `FallbackSink`.
//!
//! - Goal: show how to fan out events to multiple sinks and fall back when a primary sink fails.
//! - Behavior: Multicast sends each event to memory + log; Fallback routes to a secondary sink on error.
//! - Expected output: printed events from the log sink, memory sink counts, and streamed events.
//! - Run with: `cargo run --example telemetry_composition`

use ninelives::prelude::*;
use std::time::Duration;
use tower::{Service, ServiceBuilder, ServiceExt};

const STREAM_PROCESSING_POLL_DELAY_MS: u64 = 50; // Give the streaming task time to drain events before printing

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    println!("=== Telemetry Sink Composition Example ===\n");

    // Create multiple sinks
    let memory_sink = MemorySink::new();
    let log_sink = LogSink;

    // Example 1: MulticastSink - send events to both sinks
    println!("Example 1: MulticastSink (events go to both memory and log)");

    let multicast = MulticastSink::new(memory_sink.clone(), log_sink);

    let retry_policy = RetryPolicy::<std::io::Error>::builder()
        .max_attempts(2)
        .backoff(Backoff::constant(Duration::from_millis(50)))
        .build()?;

    let retry_layer = retry_policy.into_layer().with_sink(multicast);

    let attempt = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let attempt_clone = attempt.clone();
    let mut svc = ServiceBuilder::new().layer(retry_layer).service_fn(move |_req: &str| {
        let count = attempt_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        async move {
            if count == 0 {
                Err(std::io::Error::new(std::io::ErrorKind::Other, "first attempt failed"))
            } else {
                Ok::<_, std::io::Error>("success")
            }
        }
    });

    let result = svc.ready().await?.call("test").await?;
    println!("\nResult: {:?}", result);

    println!("\nEvents captured in MemorySink:");
    for event in memory_sink.events() {
        println!("  - {}", event);
    }

    // Example 2: FallbackSink - primary with fallback
    println!("\n\nExample 2: FallbackSink (try primary, fallback on error)");

    let memory_sink2 = MemorySink::new();
    let fallback_memory = MemorySink::new();

    // Note: In a real scenario, the primary sink might be a remote service that can fail
    // For this example, we'll use MemorySink for both (which never fails)
    let fallback = FallbackSink::new(memory_sink2.clone(), fallback_memory.clone());

    let circuit_config = CircuitBreakerConfig::new(3, Duration::from_secs(10), 1)?;

    let circuit_layer = CircuitBreakerLayer::new(circuit_config)?.with_sink(fallback);

    let mut svc = ServiceBuilder::new()
        .layer(circuit_layer)
        .service_fn(|_req: &str| async move { Ok::<_, std::io::Error>("response") });

    let response = svc.ready().await?.call("test").await?;
    println!("Response: {}", response);

    println!("Events in primary MemorySink: {}", memory_sink2.len());
    for event in memory_sink2.events() {
        println!("  - {}", event);
    }

    println!("\nEvents in fallback MemorySink: {}", fallback_memory.len());
    if fallback_memory.is_empty() {
        println!("  (empty - primary succeeded)");
    }

    // Example 3: Complex composition - multicast with streaming
    println!("\n\nExample 3: Complex composition (multicast + streaming)");

    let streaming_sink = StreamingSink::new(100);
    let mut receiver = streaming_sink.subscribe();
    let memory_sink3 = MemorySink::new();

    let complex_sink = MulticastSink::new(streaming_sink, memory_sink3.clone());

    let timeout_layer = TimeoutLayer::new(Duration::from_millis(100))?.with_sink(complex_sink);

    let mut svc = ServiceBuilder::new()
        .layer(timeout_layer)
        .service_fn(|_req: &str| async move { Ok::<_, std::io::Error>("fast response") });

    // Spawn event printer
    let event_printer = tokio::spawn(async move {
        println!("\nStreaming events:");
        while let Ok(event) = receiver.recv().await {
            println!("  [stream] {}", event);
        }
        tracing::trace!("streaming receiver closed; shutting down printer task");
    });

    let response = svc.ready().await?.call("test").await?;
    println!("Streaming example response: {}", response);

    tokio::time::sleep(Duration::from_millis(STREAM_PROCESSING_POLL_DELAY_MS)).await;

    println!("\nAlso stored in memory:");
    for event in memory_sink3.events() {
        println!("  [memory] {}", event);
    }

    // Cleanup
    drop(svc); // drop sends to close the streaming channel
    if let Err(e) = event_printer.await {
        eprintln!("event printer task ended with error: {e}");
    }

    println!("\n✓ Telemetry composition working successfully!");

    Ok(())
}
