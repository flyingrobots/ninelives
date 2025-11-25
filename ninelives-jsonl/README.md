# ninelives-jsonl

JSONL telemetry sink for `ninelives` (optional).

## Usage

```toml
ninelives = "0.2"
ninelives-jsonl = { path = "../ninelives-jsonl", features = ["async-fs"] }
```

```rust
use ninelives::telemetry::NonBlockingSink;
use ninelives_jsonl::JsonlSink;
# async fn run() -> Result<(), Box<dyn std::error::Error>> {
let raw = JsonlSink::new("./policy-events.jsonl");
let sink = NonBlockingSink::with_capacity(raw, 1024);
# Ok(()) }
```

## Notes
- Writes one JSON line per event (payload: Debug string of the event).
- Use `NonBlockingSink` to avoid blocking hot paths.
- Enable `async-fs` feature to perform async file writes via tokio.
