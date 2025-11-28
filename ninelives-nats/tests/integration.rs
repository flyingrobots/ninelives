use ninelives::telemetry::{PolicyEvent, RetryEvent};
use ninelives_nats::NatsSink;

// Requires NATS running and env NINE_LIVES_TEST_NATS_URL set, e.g. nats://127.0.0.1:4222
#[tokio::test]
#[ignore]
async fn publishes_events_to_nats() {
    let url = std::env::var("NINE_LIVES_TEST_NATS_URL").expect("set NINE_LIVES_TEST_NATS_URL");
    let subject = "policy.events";

    let client = nats::asynk::connect(url.clone()).await.expect("connect nats");
    let mut sink = NatsSink::new(client.clone(), subject);

    // subscribe before publishing to avoid drops
    let sub_client = nats::asynk::connect(url).await.expect("connect nats sub");
    let sub = sub_client.subscribe(subject).await.expect("subscribe");

    let event = PolicyEvent::Retry(RetryEvent::Attempt {
        attempt: 1,
        delay: std::time::Duration::from_millis(50),
    });
    sink.call(event).await.unwrap();

    let msg = sub.next().await.expect("message");
    let payload: serde_json::Value = serde_json::from_slice(&msg.data).unwrap();
    assert_eq!(payload["kind"], "retry_attempt");
}
