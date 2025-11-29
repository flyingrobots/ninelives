use ninelives::telemetry::{
    BulkheadEvent, CircuitBreakerEvent, PolicyEvent, RequestOutcome, RetryEvent, TimeoutEvent,
};
use ninelives_prometheus::PrometheusSink;
use prometheus::Registry;
use tower_service::Service;

fn get_counter_value(registry: &Registry, event_type: &str) -> Option<f64> {
    let metric_families = registry.gather();
    let events_metric = metric_families.iter().find(|mf| mf.get_name() == "ninelives_events_total");

    if let Some(metric) = events_metric {
        if let Some(m) = metric.get_metric().iter().find(|m| {
            m.get_label().iter().any(|l| l.get_name() == "event" && l.get_value() == event_type)
        }) {
            if let Some(c) = m.get_counter().as_ref() {
                return Some(c.value());
            }
        }
    }
    None
}

#[tokio::test]
async fn test_retry_variants() {
    let registry = Registry::new();
    let mut sink = PrometheusSink::new(registry.clone()).expect("Failed to create PrometheusSink");

    let cases = [
        (
            PolicyEvent::Retry(RetryEvent::Attempt {
                attempt: 1,
                delay: std::time::Duration::from_millis(50),
            }),
            "attempt",
        ),
        (
            PolicyEvent::Retry(RetryEvent::Exhausted {
                total_attempts: 3,
                total_duration: std::time::Duration::from_millis(150),
            }),
            "exhausted",
        ),
    ];

    for (event, label) in cases {
        sink.call(event).await.expect("Failed to call sink with retry event");
        let val = get_counter_value(&registry, label).expect("metric missing");
        assert_eq!(val, 1.0);
    }
}

#[tokio::test]
async fn test_circuit_breaker_variants() {
    let registry = Registry::new();
    let mut sink = PrometheusSink::new(registry.clone()).expect("Failed to create PrometheusSink");

    let cases = [
        (PolicyEvent::CircuitBreaker(CircuitBreakerEvent::Opened { failure_count: 5 }), "opened"),
        (PolicyEvent::CircuitBreaker(CircuitBreakerEvent::HalfOpen), "half_open"),
        (PolicyEvent::CircuitBreaker(CircuitBreakerEvent::Closed), "closed"),
    ];

    for (event, label) in cases {
        sink.call(event).await.expect("Failed to call sink with CB event");
        let val = get_counter_value(&registry, label).expect("metric missing");
        assert_eq!(val, 1.0);
    }
}

#[tokio::test]
async fn test_bulkhead_variants() {
    let registry = Registry::new();
    let mut sink = PrometheusSink::new(registry.clone()).expect("Failed to create PrometheusSink");

    let cases = [
        (
            PolicyEvent::Bulkhead(BulkheadEvent::Acquired { active_count: 1, max_concurrency: 2 }),
            "acquired",
        ),
        (
            PolicyEvent::Bulkhead(BulkheadEvent::Rejected { active_count: 2, max_concurrency: 2 }),
            "rejected",
        ),
    ];

    for (event, label) in cases {
        sink.call(event).await.expect("Failed to call sink with Bulkhead event");
        let val = get_counter_value(&registry, label).expect("metric missing");
        assert_eq!(val, 1.0);
    }
}

#[tokio::test]
async fn test_timeout_event_increments() {
    let registry = Registry::new();
    let mut sink = PrometheusSink::new(registry.clone()).expect("Failed to create PrometheusSink");

    let event =
        PolicyEvent::Timeout(TimeoutEvent::Occurred { timeout: std::time::Duration::from_secs(1) });

    assert_eq!(get_counter_value(&registry, "occurred"), Some(0.0));
    sink.call(event.clone()).await.expect("Failed to call sink with Timeout event");
    sink.call(event).await.expect("Failed to call sink with Timeout event");
    assert_eq!(get_counter_value(&registry, "occurred"), Some(2.0));
}

#[tokio::test]
async fn test_request_outcome_event_increments() {
    let registry = Registry::new();
    let mut sink = PrometheusSink::new(registry.clone()).expect("Failed to create PrometheusSink");

    let event = PolicyEvent::Request(RequestOutcome::Success {
        duration: std::time::Duration::from_millis(100),
    });

    assert_eq!(get_counter_value(&registry, "success"), Some(0.0));
    sink.call(event.clone()).await.expect("Failed to call sink with Request event");
    sink.call(event).await.expect("Failed to call sink with Request event");
    assert_eq!(get_counter_value(&registry, "success"), Some(2.0));
}
