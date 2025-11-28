# ninelives-nats

NATS telemetry sink for the `ninelives` resilience library. Bring your own async
`async_nats::Client`; events are serialized to JSON and published to a
subject of your choice.

## Usage

```toml
ninelives = "0.3"
ninelives-nats = { path = "../ninelives-nats" }
async-nats = "0.36"
```

```rust
use ninelives::telemetry::NonBlockingSink;
use ninelives_nats::NatsSink;

# async fn run() -> Result<(), Box<dyn std::error::Error>> {
let client = async_nats::connect("nats://127.0.0.1:4222").await?;
let raw = NatsSink::new(client, "policy.events");
let sink = NonBlockingSink::with_capacity(raw, 1024);
// attach via .with_sink(...) on your policy layer
# Ok(()) }
```

## Recipe
- Publish every `PolicyEvent` to subject `policy.events`.
- Wrap with `NonBlockingSink` to keep request paths non-blocking.
- Subscribe with any NATS client to power an Observer or downstream pipeline.

## Integration Test (real NATS)

```bash
cd ninelives-nats
docker compose up -d      # starts nats:2.10-alpine on 4222
export NINE_LIVES_TEST_NATS_URL=nats://127.0.0.1:4222
cargo test -- --ignored   # runs tests/integration.rs
```

The ignored test publishes a `PolicyEvent` and asserts the JSON payload is
received on the configured subject.
