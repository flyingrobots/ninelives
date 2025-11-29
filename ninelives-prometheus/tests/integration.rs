use ninelives::telemetry::{
    BulkheadEvent, CircuitBreakerEvent, PolicyEvent, RequestOutcome, RetryEvent, TimeoutEvent,
};
use ninelives_prometheus::PrometheusSink;
use prometheus::Registry;
use tower_service::Service;

fn get_counter_value(registry: &Registry, event_type: &str) -> f64 {
    let metric_families = registry.gather();
    let events_metric = metric_families
        .iter()
        .find(|mf| mf.get_name() == "ninelives_events_total");

    if let Some(metric) = events_metric {
        if let Some(m) = metric.get_metric().iter().find(|m| {
            m.get_label()
                .iter()
                .any(|l| l.get_name() == "event" && l.get_value() == event_type)
        }) {
            return m.get_counter().get_value();
        }
    }
    0.0
}

#[tokio::test]
async fn test_retry_event_increments() {
    let registry = Registry::new();
    let mut sink = PrometheusSink::new(registry.clone());

    let event = PolicyEvent::Retry(RetryEvent::Attempt {
        attempt: 1,
        delay: std::time::Duration::from_millis(50),
    });

    // Initial value
    assert_eq!(get_counter_value(&registry, "Retry::Attempt"), 0.0);

    // Call once
    sink.call(event.clone())
        .await
        .expect("Failed to call sink with retry event");
    assert_eq!(get_counter_value(&registry, "Retry::Attempt"), 1.0);

    // Call again
    sink.call(event)
        .await
        .expect("Failed to call sink with retry event");
    assert_eq!(get_counter_value(&registry, "Retry::Attempt"), 2.0);
}

#[tokio::test]
async fn test_circuit_breaker_event_increments() {
    let registry = Registry::new();
    let mut sink = PrometheusSink::new(registry.clone());

    let event = PolicyEvent::CircuitBreaker(CircuitBreakerEvent::Opened { failure_count: 5 });

    assert_eq!(get_counter_value(&registry, "CircuitBreaker::Opened"), 0.0);

    sink.call(event.clone())
        .await
        .expect("Failed to call sink with CB event");
    sink.call(event)
        .await
        .expect("Failed to call sink with CB event");

    assert_eq!(get_counter_value(&registry, "CircuitBreaker::Opened"), 2.0);
}

#[tokio::test]
async fn test_bulkhead_event_increments() {
    let registry = Registry::new();
    let mut sink = PrometheusSink::new(registry.clone());

    let event =
        PolicyEvent::Bulkhead(BulkheadEvent::Rejected { active_count: 10, max_concurrency: 10 });

    assert_eq!(get_counter_value(&registry, "Bulkhead::Rejected"), 0.0);

    sink.call(event.clone())
        .await
        .expect("Failed to call sink with Bulkhead event");
    sink.call(event)
        .await
        .expect("Failed to call sink with Bulkhead event");

    assert_eq!(get_counter_value(&registry, "Bulkhead::Rejected"), 2.0);
}

#[tokio::test]
async fn test_timeout_event_increments() {
    let registry = Registry::new();
    let mut sink = PrometheusSink::new(registry.clone());

    let event =
        PolicyEvent::Timeout(TimeoutEvent::Occurred { timeout: std::time::Duration::from_secs(1) });

    assert_eq!(get_counter_value(&registry, "Timeout::Occurred"), 0.0);

    sink.call(event.clone())
        .await
        .expect("Failed to call sink with Timeout event");
    sink.call(event)
        .await
        .expect("Failed to call sink with Timeout event");

    assert_eq!(get_counter_value(&registry, "Timeout::Occurred"), 2.0);
}

#[tokio::test]
async fn test_request_outcome_event_increments() {
    let registry = Registry::new();
    let mut sink = PrometheusSink::new(registry.clone());

    let event = PolicyEvent::Request(RequestOutcome::Success {
        duration: std::time::Duration::from_millis(100),
    });

    assert_eq!(get_counter_value(&registry, "Request::Success"), 0.0);

    sink.call(event.clone())
        .await
        .expect("Failed to call sink with Request event");
    sink.call(event)
        .await
        .expect("Failed to call sink with Request event");

    assert_eq!(get_counter_value(&registry, "Request::Success"), 2.0);
}