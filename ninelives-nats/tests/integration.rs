#![cfg(feature = "client")]
use ninelives::telemetry::{PolicyEvent, RetryEvent};
use ninelives_nats::NatsSink;
use testcontainers::{clients::Cli, core::WaitFor, images::generic::GenericImage, Container};

fn start_nats() -> (Cli, Container<GenericImage>, String) {
    let docker = Cli::default();
    let image = GenericImage::new("nats", "2.10.8-alpine")
        .with_wait_for(WaitFor::message("Server is ready"));
    let container = docker.run(image);
    let host_port = container.get_host_port_ipv4(4222);
    let addr = format!("nats://127.0.0.1:{}", host_port);
    (docker, container, addr)
}

#[tokio::test]
#[ignore]
async fn publishes_events_to_nats() {
    let (_cli, _node, addr) = start_nats();

    let mut sink = NatsSink::new(addr.clone(), "policy.events").expect("sink");

    // Subscribe before publishing to avoid missing messages
    let sub_conn = nats::asynk::connect(addr.clone()).await.unwrap();
    let sub = sub_conn.subscribe("policy.events").await.unwrap();

    // Publish an event
    let event = PolicyEvent::Retry(RetryEvent::Attempt {
        attempt: 1,
        delay: std::time::Duration::from_millis(50),
    });
    sink.call(event).await.unwrap();

    // Assert delivery
    let msg = sub.next().await.expect("message");
    let payload: serde_json::Value = serde_json::from_slice(&msg.data).unwrap();
    assert_eq!(payload["kind"], "retry_attempt");
}
