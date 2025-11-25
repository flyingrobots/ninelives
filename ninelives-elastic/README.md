# ninelives-elastic

Optional Elasticsearch telemetry sink for `ninelives`.

## Usage

```toml
ninelives = "0.2"
ninelives-elastic = { path = "../ninelives-elastic", features = ["client"] }
```

```rust
use ninelives::telemetry::NonBlockingSink;
use ninelives_elastic::ElasticSink;
# async fn run() -> Result<(), Box<dyn std::error::Error>> {
let raw = ElasticSink::new("http://localhost:9200", "policy-events")?;
let sink = NonBlockingSink::with_capacity(raw, 1024);
# Ok(()) }
```

## Recipe
- Index `PolicyEvent` documents into Elasticsearch.
- Wrap with `NonBlockingSink` to avoid blocking request paths.
- Use ILM/rollover on the index in production.

## Features
- `client` (off by default): pulls in `elasticsearch` client + `reqwest` + `serde_json`.
