use ninelives::telemetry::{PolicyEvent, RetryEvent};
use ninelives_kafka::KafkaSink;
use rdkafka::{
    admin::{AdminClient, AdminOptions, NewTopic, TopicReplication},
    client::DefaultClientContext,
    consumer::{Consumer, StreamConsumer},
    producer::{FutureProducer, Producer},
    ClientConfig, Message,
};
use std::time::Duration;
use tower_service::Service;
use uuid::Uuid;

// Requires Kafka running. If NINE_LIVES_TEST_KAFKA_BROKERS is unset, the test skips.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn publishes_events_to_kafka() {
    let brokers = match std::env::var("NINE_LIVES_TEST_KAFKA_BROKERS") {
        Ok(v) => v,
        Err(_) => {
            eprintln!("skipping: set NINE_LIVES_TEST_KAFKA_BROKERS (e.g. 127.0.0.1:9092)");
            return;
        }
    };

    let unique_id = Uuid::new_v4().to_string();
    let topic_name = format!("policy.events.test.{}", unique_id);
    let consumer_group_id = format!("ninelives-test-group-{}", unique_id);

    // --- Admin Client to create/delete topic ---
    let admin_client: AdminClient<DefaultClientContext> = ClientConfig::new()
        .set("bootstrap.servers", &brokers)
        .create()
        .expect("failed to create Kafka AdminClient");

    let topic = NewTopic::new(&topic_name, 1, TopicReplication::Fixed(1));
    admin_client
        .create_topics(&[topic], &AdminOptions::new())
        .await
        .expect("Failed to create topic");

    // --- Producer ---
    let producer: FutureProducer = ClientConfig::new()
        .set("bootstrap.servers", &brokers)
        .set("message.timeout.ms", "10000")
        .create()
        .expect("failed to create Kafka producer");

    let mut sink = KafkaSink::new(producer.clone(), &topic_name);
    let event =
        PolicyEvent::Retry(RetryEvent::Attempt { attempt: 1, delay: Duration::from_millis(50) });

    sink.call(event.clone()).await.expect("failed to sink policy event to Kafka");

    // Flush producer to ensure message is sent
    producer.flush(Duration::from_secs(5)).expect("Failed to flush producer");

    // --- Consumer ---
    let consumer: StreamConsumer = ClientConfig::new()
        .set("group.id", &consumer_group_id) // Unique consumer group
        .set("bootstrap.servers", &brokers)
        .set("auto.offset.reset", "earliest")
        .set("enable.auto.commit", "false") // Disable auto-commit
        .create()
        .expect("failed to create Kafka consumer");

    consumer.subscribe(&[&topic_name]).expect("Failed to subscribe to topic");

    // Wait for partition assignment
    let mut assigned = false;
    for _ in 0..10 {
        if consumer.assignment().expect("Failed to get assignment").count() > 0 {
            assigned = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    assert!(assigned, "Consumer failed to get partition assignment within timeout");

    let msg = tokio::time::timeout(
        Duration::from_secs(10), // Timeout for receiving message
        consumer.recv(),
    )
    .await
    .expect("timeout waiting for message") // Timeout from tokio::time::timeout
    .expect("failed to receive message from Kafka"); // Error from consumer.recv()

    let payload = msg.payload().expect("message has no payload");
    let val: serde_json::Value =
        serde_json::from_slice(payload).expect("failed to parse payload as JSON");

    assert_eq!(val["kind"], "retry_attempt");
    assert_eq!(val["attempt"], 1, "attempt field mismatch");
    assert_eq!(val["delay_ms"], 50, "delay_ms field mismatch");

    // --- Cleanup ---
    // Ensure consumer is unsubscribed and producer is dropped
    consumer.unsubscribe();
    drop(producer); // Producer is dropped implicitly when it goes out of scope

    admin_client
        .delete_topics(&[&topic_name], &AdminOptions::new())
        .await
        .expect("Failed to delete topic");
}
