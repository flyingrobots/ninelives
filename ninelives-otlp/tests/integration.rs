//! Integration test for ninelives-otlp sink against a real OTLP collector.

use ninelives::telemetry::{PolicyEvent, RetryEvent};
use ninelives_otlp::OtlpSink;
use opentelemetry_otlp::WithExportConfig;
use std::time::Duration;
use tower_service::Service;

// Requires an OTLP collector listening on HTTP. If NINE_LIVES_TEST_OTLP_ENDPOINT is unset, skip.
#[tokio::test]
async fn publishes_events_to_otlp() {
    let endpoint = match std::env::var("NINE_LIVES_TEST_OTLP_ENDPOINT") {
        Ok(v) => v,
        Err(_) => {
            eprintln!("skipping: set NINE_LIVES_TEST_OTLP_ENDPOINT (e.g. http://127.0.0.1:4318)");
            return;
        }
    };

    // Build an OTLP HTTP log exporter hitting the collector
    let exporter = opentelemetry_otlp::LogExporter::builder()
        .with_http()
        .with_endpoint(endpoint)
        .with_timeout(Duration::from_secs(5))
        .build()
        .expect("build otlp exporter");

    // Wire into a logger provider
    let processor = opentelemetry_sdk::logs::BatchLogProcessor::builder(exporter)
        .with_batch_config(
            opentelemetry_sdk::logs::BatchConfigBuilder::default()
                .with_scheduled_delay(Duration::from_millis(200))
                .build(),
        )
        .build();

    let provider =
        opentelemetry_sdk::logs::SdkLoggerProvider::builder().with_log_processor(processor).build();

    let mut sink = OtlpSink::new(provider.clone());

    let event =
        PolicyEvent::Retry(RetryEvent::Attempt { attempt: 1, delay: Duration::from_millis(50) });

    sink.call(event).await.expect("send event");

    // Flush to ensure export completes
    provider.force_flush().unwrap();
    provider.shutdown().unwrap();
}
