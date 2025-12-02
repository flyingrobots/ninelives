# ninelives-jsonl

JSONL telemetry sink for the `ninelives` resilience library. Writes one JSON event per line to a file you choose.

## Usage

```toml
ninelives = "0.3"
# For published crate users:
ninelives-jsonl = "0.3"

# For local monorepo development:
# ninelives-jsonl = { path = "../ninelives-jsonl" }
```

```rust
use ninelives::telemetry::NonBlockingSink;
use ninelives_jsonl::JsonlSink;

let raw = JsonlSink::new("/var/log/policy-events.jsonl");
let sink = NonBlockingSink::with_capacity(raw, 1024);
```

## Test

```bash
cargo test -p ninelives-jsonl
```

The integration test writes a `PolicyEvent` to a temp file and asserts the line contains `retry_attempt`.
