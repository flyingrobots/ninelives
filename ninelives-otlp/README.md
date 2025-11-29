# ninelives-otlp

OTLP telemetry sink for the `ninelives` resilience library. Bring your own `opentelemetry::logs::Logger`; events are emitted as log records (export destination is up to your pipeline).

## Usage

```toml
ninelives = "0.3"
ninelives-otlp = { path = "../ninelives-otlp" }
opentelemetry = { version = "0.22", features = ["logs"] }
opentelemetry_sdk = { version = "0.22", features = ["logs"] }
```

```rust
use ninelives::telemetry::NonBlockingSink;
use ninelives_otlp::OtlpSink;
use opentelemetry::logs::Logger;
use opentelemetry_sdk::logs::{LoggerProvider, SimpleLogProcessor};
use opentelemetry_sdk::export::logs::LogExporter;

// build your own exporter/pipeline, then get a Logger
# struct NoopExporter;
# impl LogExporter for NoopExporter {
#     fn export(&self, _: Vec<opentelemetry_sdk::export::logs::LogData>) -> opentelemetry::export::logs::ExportResult { opentelemetry::export::logs::ExportResult::Success }
#     fn shutdown(&self) {}
# }
let provider = LoggerProvider::builder()
    .with_log_processor(SimpleLogProcessor::new(Box::new(NoopExporter)))
    .build();
let logger: Logger = provider.logger_builder("ninelives-otlp").build();
let raw = OtlpSink::new(logger);
let sink = NonBlockingSink::with_capacity(raw, 1024);
```

## Test

```bash
cargo test -p ninelives-otlp
```

The integration test builds an in-memory exporter and asserts a log record is emitted.
