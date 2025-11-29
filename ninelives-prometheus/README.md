# ninelives-prometheus

Prometheus metrics sink for the `ninelives` resilience library. Bring your own `prometheus::Registry`; the sink registers `ninelives_events_total{policy,event}` counters and increments them for each policy event.

## Usage

```toml
ninelives = "0.3"
ninelives-prometheus = { path = "../ninelives-prometheus" }
prometheus = "0.13"
```

```rust
use ninelives::telemetry::NonBlockingSink;
use ninelives_prometheus::PrometheusSink;
use prometheus::Registry;

let registry = Registry::new();
let raw = PrometheusSink::new(registry);
let sink = NonBlockingSink::with_capacity(raw, 1024);
// expose registry via /metrics using prometheus::TextEncoder
```

## Test

```bash
cargo test -p ninelives-prometheus
```

The integration test verifies a `PolicyEvent` increments the counter and is present in gathered metrics.
