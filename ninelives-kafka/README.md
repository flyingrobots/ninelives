# ninelives-kafka

Optional Kafka telemetry sink for `ninelives`.

## Usage

```toml
ninelives = "0.2"
ninelives-kafka = { path = "../ninelives-kafka", features = ["client"] }
```

```rust
use ninelives::telemetry::NonBlockingSink;
use ninelives_kafka::KafkaSink;
# async fn run() -> Result<(), Box<dyn std::error::Error>> {
let raw = KafkaSink::new("localhost:9092", "policy-events")?;
let sink = NonBlockingSink::with_capacity(raw, 2048);
# Ok(()) }
```

## Recipe
- Serialize `PolicyEvent` to JSON and send to Kafka topic.
- Wrap with `NonBlockingSink` to protect request latency.

## Features
- `client` (off by default): pulls in `rdkafka` & serde_json to actually emit.
