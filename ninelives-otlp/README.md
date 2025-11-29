# ninelives-otlp

OTLP telemetry sink for the `ninelives` resilience library. Bring your own `opentelemetry_sdk::logs::SdkLoggerProvider`; events are emitted as OTLP log records (export destination is up to your pipeline).

## Usage

```toml
ninelives = "0.3"
ninelives-otlp = { path = "../ninelives-otlp" }
opentelemetry = { version = "0.31", features = ["logs"] }
opentelemetry_sdk = { version = "0.31", features = ["logs", "rt-tokio"] }
opentelemetry-otlp = { version = "0.31", features = ["logs", "http-proto", "reqwest-client", "reqwest-rustls"] }
```

```rust
use ninelives::telemetry::NonBlockingSink;
use ninelives_otlp::OtlpSink;
use opentelemetry_otlp::WithExportConfig;
use std::time::Duration;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let exporter = opentelemetry_otlp::new_exporter()
        .http()
        .with_endpoint("http://127.0.0.1:4318")
        .with_timeout(Duration::from_secs(5))
        .build_log_exporter()
        .await?;

    let processor = opentelemetry_sdk::logs::BatchLogProcessor::builder(
        exporter,
        opentelemetry_sdk::runtime::Tokio,
    )
    .build();

    let provider = opentelemetry_sdk::logs::SdkLoggerProvider::builder()
        .with_log_processor(processor)
        .build();

    let raw = OtlpSink::new(provider);
    let sink = NonBlockingSink::with_capacity(raw, 1024);
    // attach via .with_sink(...) on your policy layer
    Ok(())
}
```

## Integration Test (real OTLP collector)

```bash
cd ninelives-otlp
docker compose up -d
export NINE_LIVES_TEST_OTLP_ENDPOINT=http://127.0.0.1:4318
cargo test -p ninelives-otlp
docker compose down
```

The test builds an OTLP HTTP log exporter pointed at the collector from `docker-compose.yml` and verifies a `PolicyEvent` is exported successfully.
