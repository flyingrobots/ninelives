use async_nats::Client;
use futures_util::StreamExt;
use ninelives::telemetry::{PolicyEvent, RetryEvent};
use ninelives_nats::NatsSink;
use tower_service::Service;

// Requires NATS running. If NINE_LIVES_TEST_NATS_URL is unset, the test skips.
#[tokio::test]
async fn publishes_events_to_nats() {
    let url = match std::env::var("NINE_LIVES_TEST_NATS_URL") {
        Ok(v) => v,
        Err(_) => {
            eprintln!("skipping: set NINE_LIVES_TEST_NATS_URL (e.g. nats://127.0.0.1:4222)");
            return;
        }
    };
    let subject = "policy.events";

    let client: Client = async_nats::connect(url.clone()).await.expect("connect nats");
    let mut sink = NatsSink::new(client.clone(), subject);

    // subscribe before publishing to avoid drops
    let mut sub = client.subscribe(subject).await.expect("subscribe");

    let event = PolicyEvent::Retry(RetryEvent::Attempt {
        attempt: 1,
        delay: std::time::Duration::from_millis(50),
    });
    sink.call(event).await.unwrap();

    let msg = sub.next().await.expect("message");
    let payload: serde_json::Value = serde_json::from_slice(&msg.payload).unwrap();
    assert_eq!(payload["kind"], "retry_attempt");
}
