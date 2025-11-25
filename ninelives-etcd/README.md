# ninelives-etcd

Optional etcd telemetry sink for `ninelives`.

## Usage

```toml
ninelives = "0.1"
ninelives-etcd = { path = "../ninelives-etcd", features = ["client"] }
```

```rust
use ninelives::telemetry::NonBlockingSink;
use ninelives_etcd::EtcdSink;
# async fn run() -> Result<(), Box<dyn std::error::Error>> {
let raw = EtcdSink::new("http://127.0.0.1:2379", "policy/events").await?;
let sink = NonBlockingSink::with_capacity(raw, 512);
# Ok(()) }
```

## Behavior
- Stores each event under key `prefix/<nanos>` with value `{:?}` of the event.
- Wrap with `NonBlockingSink` to avoid blocking request paths.

## Features
- `client` (off by default): pulls in `etcd-client` + tokio.
