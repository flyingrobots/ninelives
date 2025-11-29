# ninelives-cookbook

Ready-to-use policy recipes and runnable examples for the `ninelives` resilience library.

## Install
```toml
ninelives = "0.3"
ninelives-cookbook = { path = "../ninelives-cookbook" }
```

## Recipes (library)

| Recipe | When to use | What it does | Defaults & Rationale |
| --- | --- | --- | --- |
| `retry_fast` | Transient errors, low latency tolerance | 3 attempts, exponential backoff starting at 50ms, full jitter | Keeps tail low; 3 shots avoids thundering herd; jitter to de-sync callers |
| `timeout_p95` | Guard the 95th percentile; fast fail over slow deps | Timeout at 300ms | Good starting SLO guard; tune per-service p95 |
| `bulkhead_isolate(max)` | Protect shared resources from overload | Concurrency cap with immediate rejection | Favors fail-fast over queue buildup |
| `api_guardrail` | External API calls | Timeout + CircuitBreaker + Bulkhead | Stops slow/failed deps from cascading; breaker tuned for flapping |
| `reliable_read` | Read-heavy, wants fast path but solid fallback | Fast retry+timeout OR slower generous stack | Balances latency and success rate |
| `hedged_read` | Reduce tail latency (“happy eyeballs”) | Fork-join two differently tuned stacks | Races fast vs steady to cut p99 |
| `hedged_then_fallback` | “God tier” safety: race, then fall back | Hedge two fast paths, fallback to sturdy stack | High availability under variance and failure |
| `sensible_defaults(max)` | General I/O starter pack | Timeout + Retry + Bulkhead | Safe defaults; pass your concurrency budget |

### Adaptive knobs

All recipes above support runtime tuning without restarting. Use these methods:
- `retry_fast`: call `policy.adaptive_max_attempts()`, `adaptive_backoff_base()`, `adaptive_jitter()` to tune retry behavior live.
- `timeout_p95`: call `policy.adaptive_duration()` to adjust the timeout.
- `api_guardrail` / `hedged_then_fallback`: component policies are adaptive-capable; wire their handles into your builder (see `control_plane` example).
- `bulkhead_isolate`: call `policy.adaptive_max_concurrent(new_cap)` to raise the concurrency limit (up to system resource limits).

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
cargo run -p ninelives-cookbook --example timeout_algebraic_composition
cargo run -p ninelives-cookbook --example control_plane   # control-plane quickstart
```

(Examples mirror the recipes and show end-to-end usage.)

## Why a separate crate?
To keep the core `ninelives` crate lean while providing richer, opinionated recipes and runnable samples without pulling extra weight into core builds.

## Notes on tuning
- Timeouts: set to your service’s p95/p99, not the defaults here.
- Backoff: increase base if your downstream rate-limits; decrease if you need faster recovery.
- Bulkhead: set `max_in_flight` to sustainable concurrency for your dependency, not CPU cores.
- Hedge: works best when downstream variance is high; disable if backend can’t handle extra load.
