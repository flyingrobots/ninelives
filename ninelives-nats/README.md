# ninelives-nats

Optional NATS telemetry sink for the `ninelives` resilience library.

## Usage

```toml
ninelives = "0.2"
ninelives-nats = { path = "../ninelives-nats", features = ["client"] }
```

```rust
use ninelives::telemetry::NonBlockingSink;
use ninelives_nats::NatsSink;

# async fn run() -> Result<(), Box<dyn std::error::Error>> {
let raw = NatsSink::new("nats://127.0.0.1:4222", "policy.events")?;
let sink = NonBlockingSink::with_capacity(raw, 1024);
// attach via .with_sink(...) on your policy layer
# Ok(()) }
```

## Recipe
- Publish every `PolicyEvent` to subject `policy.events`.
- Wrap with `NonBlockingSink` to keep request paths non-blocking.
- Subscribe with any NATS client to power an Observer or downstream pipeline.

## Features
- `client` (off by default): pulls in `nats` + `tokio` and actually publishes. Without it, the sink is a no-op but compiles fast for docs/tests.
