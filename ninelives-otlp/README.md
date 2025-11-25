# ninelives-otlp

OTLP telemetry sink for `ninelives` (placeholder). Default build is no-op; enable `client` to integrate with an OTLP collector.

```toml
ninelives = "0.1"
ninelives-otlp = { path = "../ninelives-otlp", features = ["client"] }
```

```rust
use ninelives_otlp::OtlpSink;
use ninelives::telemetry::NonBlockingSink;
let sink = NonBlockingSink::with_capacity(OtlpSink::new(), 1024);
```

Note: The current implementation is a stub; wire it to your opentelemetry pipeline as needed.
