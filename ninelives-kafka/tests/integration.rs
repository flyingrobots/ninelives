#![cfg(feature = "client")]
use ninelives::telemetry::{PolicyEvent, RetryEvent};
use ninelives_kafka::KafkaSink;
use rdkafka::{
    consumer::{Consumer, StreamConsumer},
    message::BorrowedMessage,
    ClientConfig,
};
use testcontainers::{clients::Cli, core::WaitFor, images::generic::GenericImage, Container};

fn start_redpanda() -> (Cli, Container<GenericImage>, String) {
    let docker = Cli::default();
    let image = GenericImage::new("docker.redpanda.com/redpanda/redpanda", "v23.3.8")
        .with_wait_for(WaitFor::message("Started Kafka API"));
    let container = docker.run(image);
    let port = container.get_host_port_ipv4(9092);
    let brokers = format!("127.0.0.1:{}", port);
    (docker, container, brokers)
}

#[tokio::test]
#[ignore]
async fn publishes_events_to_kafka() {
    let (_cli, _node, brokers) = start_redpanda();
    let topic = "policy.events";

    let mut sink = KafkaSink::new(brokers.clone(), topic).expect("sink");
    let event = PolicyEvent::Retry(RetryEvent::Attempt {
        attempt: 1,
        delay: std::time::Duration::from_millis(50),
    });
    sink.call(event).await.unwrap();

    let consumer: StreamConsumer = ClientConfig::new()
        .set("group.id", "test-group")
        .set("bootstrap.servers", &brokers)
        .set("auto.offset.reset", "earliest")
        .create()
        .unwrap();
    consumer.subscribe(&[topic]).unwrap();

    let msg: BorrowedMessage = consumer.recv().await.unwrap();
    let payload = msg.payload().expect("payload");
    let val: serde_json::Value = serde_json::from_slice(payload).unwrap();
    assert_eq!(val["kind"], "retry_attempt");
}
