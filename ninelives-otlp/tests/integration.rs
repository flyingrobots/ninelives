//! Integration tests for ninelives-otlp sink

use ninelives::telemetry::{PolicyEvent, RetryEvent};
use ninelives_otlp::OtlpSink;
use std::time::Duration;
use tower_service::Service;

#[tokio::test]
async fn otlp_sink_compiles_and_emits() {
    // Build a simple logger provider
    let provider = opentelemetry_sdk::logs::SdkLoggerProvider::builder().build();

    // Create the sink
    let mut sink = OtlpSink::new(provider);

    // Emit a test event
    let event =
        PolicyEvent::Retry(RetryEvent::Attempt { attempt: 1, delay: Duration::from_millis(100) });

    // Call the service (it will succeed - we're just testing it compiles/runs)
    let result = sink.call(event).await;
    assert!(result.is_ok());
}
