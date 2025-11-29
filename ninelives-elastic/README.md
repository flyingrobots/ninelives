# ninelives-elastic

Elasticsearch telemetry sink for the `ninelives` resilience library. Bring your own `elasticsearch::Elasticsearch` client; events are serialized to JSON and indexed into a target index.

## Usage

```toml
ninelives = "0.3"
ninelives-elastic = { path = "../ninelives-elastic" }
elasticsearch = { version = "8.19.0-alpha.1", default-features = false, features = ["rustls-tls"] }
```

```rust
use ninelives::telemetry::NonBlockingSink;
use ninelives_elastic::ElasticSink;
use elasticsearch::{Elasticsearch, http::transport::Transport};

# async fn run() -> Result<(), Box<dyn std::error::Error>> {
let transport = Transport::single_node("http://127.0.0.1:9200")?;
let client = Elasticsearch::new(transport);
let raw = ElasticSink::new(client, "policy-events");
let sink = NonBlockingSink::with_capacity(raw, 1024);
// attach via .with_sink(...) on your policy layer
# Ok(()) }
```

## Integration Test (real Elasticsearch)

```bash
cd ninelives-elastic
docker compose up -d      # starts elasticsearch:8.x on 9200 with security disabled
export NINE_LIVES_TEST_ELASTIC_URL=http://127.0.0.1:9200
cargo test                # runs tests/integration.rs (skips if env unset)
```

The test indexes a `PolicyEvent` into `policy-events` and asserts a hit is searchable.
