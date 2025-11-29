# ninelives-kafka

Kafka telemetry sink for the `ninelives` resilience library. Bring your own `rdkafka::producer::FutureProducer`; events are serialized to JSON and sent to your topic.

## Usage

```toml
ninelives = "0.3"
ninelives-kafka = { git = "https://github.com/your-org/ninelives", branch = "main" }
rdkafka = { version = "0.37", features = ["cmake-build"] }  # cmake-build: statically build librdkafka via cmake
```

```rust
use ninelives::telemetry::NonBlockingSink;
use ninelives_kafka::KafkaSink;
use rdkafka::{ClientConfig, producer::FutureProducer};

# async fn run() -> Result<(), Box<dyn std::error::Error>> {
let producer: FutureProducer = ClientConfig::new()
    .set("bootstrap.servers", "127.0.0.1:9092")
    .create()?;
let raw = KafkaSink::new(producer, "policy.events");
let sink = NonBlockingSink::with_capacity(raw, 1024);
// attach via .with_sink(...) on your policy layer
# Ok(()) }
```

## Integration Test (real Kafka)

```bash
cd ninelives-kafka
docker compose up -d      # starts Redpanda on 9092 (or docker-compose up -d if your Docker CLI requires it)
export NINE_LIVES_TEST_KAFKA_BROKERS=127.0.0.1:9092
cargo test                # runs tests/integration.rs (skips if env unset)
```

The integration test creates a unique Kafka topic and consumer group, publishes a `PolicyEvent` to the configured topic, and verifies that a consumer receives the message within a timeout. It asserts that the consumed payload is valid JSON and matches the expected `PolicyEvent` fields (`kind`, `attempt`, `delay_ms`). The test ensures isolation and idempotency by creating and deleting the topic, and disabling auto-commits.
