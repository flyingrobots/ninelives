use ninelives::telemetry::{PolicyEvent, RetryEvent};
use ninelives_prometheus::PrometheusSink;
use prometheus::{Encoder, Registry, TextEncoder};
use tower_service::Service;

#[tokio::test]
async fn increments_counters() {
    let registry = Registry::new();
    let mut sink = PrometheusSink::new(registry.clone());

    let event = PolicyEvent::Retry(RetryEvent::Attempt {
        attempt: 1,
        delay: std::time::Duration::from_millis(50),
    });
    sink.call(event).await.unwrap();

    let metric_families = registry.gather();
    let mut buffer = Vec::new();
    TextEncoder::new().encode(&metric_families, &mut buffer).unwrap();
    let text = String::from_utf8(buffer).unwrap();
    assert!(text.contains("ninelives_events_total"));
    assert!(text.contains("retry"));
}
