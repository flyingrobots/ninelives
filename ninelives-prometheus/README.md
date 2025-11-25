# ninelives-prometheus

In-process Prometheus metrics sink for `ninelives`.

## Usage

```toml
ninelives = "0.1"
ninelives-prometheus = { path = "../ninelives-prometheus", features = ["client"] }
```

```rust
use ninelives_prometheus::PrometheusSink;
use ninelives::telemetry::NonBlockingSink;
use prometheus::{TextEncoder, Encoder};
# async fn run() {
let raw = PrometheusSink::new();
let sink = NonBlockingSink::with_capacity(raw.clone(), 1024);
// expose metrics via HTTP
# let metric_families = raw.registry().gather();
# let mut buf = Vec::new();
# TextEncoder::new().encode(&metric_families, &mut buf).unwrap();
# }
```

## Behavior
- Increments `ninelives_events_total{policy="...",event="event"}` per PolicyEvent.
- Expose via your own HTTP endpoint using the provided registry.
- Wrap with `NonBlockingSink` to keep request paths fast.

## Features
- `client` (off by default): pulls in `prometheus` crate.
