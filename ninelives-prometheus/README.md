# ninelives-prometheus

Prometheus metrics sink for the `ninelives` resilience library. Bring your own `prometheus::Registry`; the sink registers `ninelives_events_total{policy,event}` counters and increments them for each policy event.

## Usage

```toml
ninelives = "0.3"
ninelives-prometheus = { path = "../ninelives-prometheus" }
prometheus = "0.13"
```

```rust
use ninelives::telemetry::{NonBlockingSink, PolicyEvent, RetryEvent};
use ninelives_prometheus::PrometheusSink;
use prometheus::{Registry, Encoder, TextEncoder};
use std::time::Duration;
use tower::Service;

// 1. Create Registry and Sink
let registry = Registry::new();
let prom_sink = PrometheusSink::new(registry.clone());
let mut sink = NonBlockingSink::with_capacity(prom_sink, 1024);

// 2. Emit an event (normally done automatically by policies)
let event = PolicyEvent::Retry(RetryEvent::Attempt {
    attempt: 1,
    delay: Duration::from_millis(50),
});
sink.call(event).await.unwrap();

// 3. Verify Metrics (e.g. for serving at /metrics)
let metric_families = registry.gather();
assert!(!metric_families.is_empty());

// Encode to text format
let mut buffer = vec![];
let encoder = TextEncoder::new();
encoder.encode(&metric_families, &mut buffer).unwrap();
let output = String::from_utf8(buffer).unwrap();

println!("{}", output);
// Output:
// # HELP ninelives_events_total Total number of policy events
// # TYPE ninelives_events_total counter
// ninelives_events_total{event="Retry::Attempt",policy="retry"} 1
```

## Test

```bash
cargo test -p ninelives-prometheus
```

The integration test instantiates a policy with a nonblocking sink, triggers a PolicyEvent, and verifies the ninelives_events_total counter increments with correct policy and event labels via registry.gather().