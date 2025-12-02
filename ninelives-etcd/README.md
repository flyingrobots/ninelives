# ninelives-etcd

etcd telemetry sink for the `ninelives` resilience library. Bring your own `etcd_client::Client`; events are serialized to JSON and stored under a prefix.

## Usage

```toml
ninelives = "0.3"
# Published crate usage
ninelives-etcd = "0.3"
# For local workspace development:
# ninelives-etcd = { path = "../ninelives-etcd" }
etcd-client = { version = "0.11", features = ["tls"] }
```

```rust
use ninelives::telemetry::NonBlockingSink;
use ninelives_etcd::EtcdSink;
use etcd_client::Client;

# async fn run() -> Result<(), Box<dyn std::error::Error>> {
let client = Client::connect(["http://127.0.0.1:2379"], None).await?;
let raw = EtcdSink::new("policy_events", client);
let sink = NonBlockingSink::with_capacity(raw, 1024);
// attach via .with_sink(...) on your policy layer
# Ok(()) }
```

## Integration Test (real etcd)

```bash
cd ninelives-etcd
docker compose up -d      # starts etcd on 2379
export NINE_LIVES_TEST_ETCD_ENDPOINT=http://127.0.0.1:2379
cargo test                # runs tests/integration.rs (skips if env unset)
```

The test writes a `PolicyEvent` under the prefix and asserts a key was created.
