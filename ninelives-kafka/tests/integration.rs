use ninelives::telemetry::{PolicyEvent, RetryEvent};
use ninelives_kafka::KafkaSink;
use rdkafka::{
    consumer::{Consumer, StreamConsumer},
    message::BorrowedMessage,
    producer::FutureProducer,
    ClientConfig,
};

// Requires Kafka running and env NINE_LIVES_TEST_KAFKA_BROKERS set (e.g. 127.0.0.1:9092)
#[tokio::test]
#[ignore]
async fn publishes_events_to_kafka() {
    let brokers =
        std::env::var("NINE_LIVES_TEST_KAFKA_BROKERS").expect("set NINE_LIVES_TEST_KAFKA_BROKERS");
    let topic = "policy.events";

    let producer: FutureProducer = ClientConfig::new()
        .set("bootstrap.servers", &brokers)
        .set("message.timeout.ms", "5000")
        .create()
        .expect("producer");

    let mut sink = KafkaSink::new(producer, topic);
    let event = PolicyEvent::Retry(RetryEvent::Attempt {
        attempt: 1,
        delay: std::time::Duration::from_millis(50),
    });
    sink.call(event).await.unwrap();

    let consumer: StreamConsumer = ClientConfig::new()
        .set("group.id", "ninelives-test")
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
