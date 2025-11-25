# ninelives-cookbook

Ready-to-use policy recipes and runnable examples for the `ninelives` resilience library.

## Install
```toml
ninelives = "0.1"
ninelives-cookbook = { path = "../ninelives-cookbook" }
```

## Recipes (library)
- `retry_fast`
- `timeout_p95`
- `bulkhead_isolate`
- `api_guardrail`
- `reliable_read`
- `hedged_read`
- `hedged_then_fallback`
- `sensible_defaults`

Use them like:
```rust
use ninelives_cookbook::api_guardrail;
use tower::ServiceBuilder;

let policy = api_guardrail()?;
let svc = ServiceBuilder::new().layer(policy).service_fn(|req: &str| async move { Ok::<_, std::io::Error>(req) });
```

## Examples (bin)
Run from this crate:
```bash
cargo run -p ninelives-cookbook --example retry_fast
```

(Examples mirror the recipes and show end-to-end usage.)

## Why a separate crate?
To keep the core `ninelives` crate lean while providing richer, opinionated recipes and runnable samples without pulling extra weight into core builds.
