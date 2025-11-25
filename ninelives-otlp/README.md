# ninelives-otlp

OTLP telemetry sink for the `ninelives` resilience library.

## Why
Send `PolicyEvent`s to your existing OTLP collector so you can view Nine Lives telemetry alongside service logs and traces.

## Add to Cargo.toml
```toml
ninelives = "0.1"
ninelives-otlp = { path = "../ninelives-otlp", features = ["client"] }
```

## Minimal usage
```rust
use ninelives::telemetry::NonBlockingSink;
use ninelives_otlp::OtlpSink;

// OTEL_EXPORTER_OTLP_ENDPOINT controls where logs go, e.g. http://localhost:4317
#[tokio::main]
async fn main() {
    let sink = NonBlockingSink::with_capacity(OtlpSink::new(), 1024);
    // attach with .with_sink(sink) on your policies
}
```

## Environment
- `OTEL_EXPORTER_OTLP_ENDPOINT` (e.g. `http://localhost:4317`)
- Optional: `OTEL_RESOURCE_ATTRIBUTES` to add service metadata.

## What we emit
- One OTLP Log per `PolicyEvent`
- Attributes: `component=ninelives`, `event_kind` (retry|circuit_breaker|bulkhead|timeout|request), plus event-specific fields (attempt, delay_ms, failure_count, duration_ms, etc.)
- Severity: `INFO` for normal flow, `WARN` for failures/timeouts/rejections/exhausted retries.

## Caveats
- Feature `client` must be enabled; otherwise the sink is a no-op.
- The current pipeline uses the OTLP log exporter; tracing spans are not emitted.
- Wrap with `NonBlockingSink` to avoid blocking request paths.
