# ninelives-kafka

Kafka telemetry sink for the `ninelives` resilience library. Bring your own `rdkafka::producer::FutureProducer`; events are serialized to JSON and sent to your topic.

## Usage

```toml
ninelives = "0.3"
ninelives-kafka = { path = "../ninelives-kafka" }
rdkafka = { version = "0.37", features = ["cmake-build"] }
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
docker compose up -d      # starts Redpanda on 9092
export NINE_LIVES_TEST_KAFKA_BROKERS=127.0.0.1:9092
cargo test                # runs tests/integration.rs (skips if env unset)
```

The test publishes a `PolicyEvent` and asserts the JSON payload is consumed from the topic.
