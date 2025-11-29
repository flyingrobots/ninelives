use ninelives::telemetry::{PolicyEvent, RetryEvent};
use ninelives_nats::NatsSink;
use tower_service::Service;
use futures_util::StreamExt; // Required for sub.next()
use std::time::Duration; // Required for tokio::time::timeout

/// Tests end-to-end event publishing to a NATS broker via NatsSink.
///
/// This test requires a running NATS instance. Set `NINE_LIVES_TEST_NATS_URL`
/// (e.g., `nats://127.0.0.1:4222`) to enable the test; if unset, the test
/// is skipped silently. This prevents CI failure when NATS is unavailable.
#[tokio::test]
async fn publishes_events_to_nats() {
    let url = match std::env::var("NINE_LIVES_TEST_NATS_URL") {
        Ok(v) => v,
        Err(_) => {
            eprintln!("skipping: set NINE_LIVES_TEST_NATS_URL (e.g. nats://127.0.0.1:4222)");
            return;
        }
    };
    const SUBJECT: &str = "policy.events";

    let client = async_nats::connect(url.clone())
        .await
        .expect("failed to connect to NATS broker; ensure NINE_LIVES_TEST_NATS_URL is reachable");
    let mut sink = NatsSink::new(client.clone(), SUBJECT);

    // Subscribe before publishing to prevent message drops
    let mut sub = client.subscribe(SUBJECT)
        .await
        .expect("failed to subscribe to NATS subject");

    let event = PolicyEvent::Retry(RetryEvent::Attempt {
        attempt: 1,
        delay: std::time::Duration::from_millis(50),
    });
    sink.call(event.clone())
        .await
        .expect(&format!("Failed to publish event to NATS sink: {:?}", event));

    let msg = tokio::time::timeout(
        Duration::from_secs(5), // Explicit timeout
        sub.next()
    )
    .await
    .expect("timeout waiting for published message") // Timeout from tokio::time::timeout
    .expect("stream closed without receiving message"); // Error from sub.next() if stream ends

    let payload: serde_json::Value = serde_json::from_slice(&msg.payload)
        .expect(&format!("failed to parse message payload as JSON: {:?}", msg.payload));

    // Assertions for payload content
    let kind = payload.get("kind")
        .expect("JSON payload missing 'kind' field")
        .as_str()
        .expect("JSON 'kind' field is not a string");
    assert_eq!(kind, "retry_attempt");

    let attempt = payload.get("attempt")
        .expect("JSON payload missing 'attempt' field")
        .as_u64()
        .expect("JSON 'attempt' field is not a number");
    assert_eq!(attempt, 1);

    let delay_ms = payload.get("delay_ms")
        .expect("JSON payload missing 'delay_ms' field")
        .as_u64()
        .expect("JSON 'delay_ms' field is not a number");
    assert_eq!(delay_ms, 50);
}