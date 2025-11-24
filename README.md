# Nine Lives 🐱 – Resilience for async Rust

[![CI](https://github.com/flyingrobots/ninelives/actions/workflows/ci.yml/badge.svg)](https://github.com/flyingrobots/ninelives/actions/workflows/ci.yml)
[![Docs](https://docs.rs/ninelives/badge.svg)](https://docs.rs/ninelives)
[![Crates.io](https://img.shields.io/crates/v/ninelives.svg)](https://crates.io/crates/ninelives)

Practical retries, circuit breakers, bulkheads, and timeouts that compose in a single stack. Zero `unsafe`, deterministic testing hooks, and clear invariants.

## Install
Add to `Cargo.toml`:
```toml
[dependencies]
ninelives = "0.1"
```
Requires Tokio (brings its own `tokio` dependency with `time`, `sync`, `macros` features).

### MSRV
- `rust-version = "1.70"` (per Cargo.toml). CI uses the latest stable toolchain; we aim to keep MSRV at or above 1.70 and will bump it in the changelog when required.

## Quick start (full stack)
```rust
use ninelives::prelude::*;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), ResilienceError<std::io::Error>> {
    let attempts = Arc::new(AtomicUsize::new(0));

    let stack: ResilienceStack<std::io::Error> = ResilienceStack::new()
        .timeout(Duration::from_secs(2))?
        .bulkhead(32)?
        .circuit_breaker(5, Duration::from_secs(30))?
        .retry(
            RetryPolicy::builder()
                .max_attempts(4) // total attempts = 4
                .backoff(Backoff::exponential(Duration::from_millis(200)).with_max(Duration::from_secs(2))?)
                .with_jitter(Jitter::full())
                .build()?,
        )
        .build()?;

    stack
        .execute(|| {
            let attempts = attempts.clone();
            async move {
                let n = attempts.fetch_add(1, Ordering::SeqCst);
                if n < 2 {
                    Err(ResilienceError::Inner(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "transient",
                    )))
                } else {
                    Ok::<_, ResilienceError<std::io::Error>>("ok")
                }
            }
        })
        .await?;
    Ok(())
}
```

## API overview (every public type)
- `Backoff`, `ConstantBackoff`, `LinearBackoff`, `ExponentialBackoff`, `BackoffStrategy`, `BackoffError`, `MAX_BACKOFF`  
  - Constructors: `constant`, `linear`, `exponential`.  
  - `with_max` caps delays (errors if `max` == 0 or `< base`).  
  - Invariants: attempt 0 => 0 delay; delays are non-decreasing; cap respected; saturates on overflow.
- `Jitter` (`None`, `Full`, `Equal`, `Decorrelated`)  
  - `apply(delay)` for stateless variants; `apply_stateful()` for decorrelated (AWS-style, atomic state, thread-safe).  
  - Invariants: outputs stay within documented ranges and never regress for decorrelated.
- `RetryPolicy`, `RetryPolicyBuilder`, `BuildError`  
  - Fields: `max_attempts` (total attempts), `backoff`, `jitter`, `should_retry`, `sleeper`.  
  - Only `ResilienceError::Inner` is retried; others propagate. Backoff/jitter run once per retry.
- `TimeoutPolicy`, `TimeoutError`, `MAX_TIMEOUT`  
  - `new(duration)` validates (0 < d ≤ MAX_TIMEOUT).  
  - `new_with_max(duration, max)` allows custom ceilings.  
  - Executes with `tokio::time::timeout`, returning `ResilienceError::Timeout` when elapsed ≥ timeout.
- `BulkheadPolicy`, `BulkheadError`  
  - Semaphore-based concurrency limit; `unlimited` helper. Validates capacity > 0.
- `CircuitBreakerPolicy`, `CircuitBreakerConfig`, `CircuitBreakerError`, `CircuitState`  
  - Closed → Open after threshold failures; Half-Open probes after timeout. Accepts custom clock for tests.
- `ResilienceStack`, `ResilienceStackBuilder`, `StackError`  
  - Composition order: Retry → CircuitBreaker → Bulkhead → Timeout. Any layer can be configured/omitted; defaults provided.
- Helpers: `Sleeper` trait, `TokioSleeper`, `InstantSleeper`, `TrackingSleeper`; `Clock`, `MonotonicClock`; `ResilienceError` with predicates (`is_timeout`, `is_bulkhead`, etc.).
- `prelude` module re-exports the above for ergonomic `use ninelives::prelude::*;`.

## Invariants (design contract)
- Backoff: attempt 0 = 0; non-decreasing; capped by `with_max` and `MAX_BACKOFF`; overflow saturates.  
- Jitter: respects documented ranges; decorrelated in `[max(prev, base), min(max, 3*prev)]`, thread-safe.  
- Retry: attempts ≤ `max_attempts`; only `Inner` retried; sleeper invoked once per retry.  
- Timeout: duration > 0 and ≤ max; timeout errors include elapsed and configured timeout.  
- Stack: layer order is fixed (Retry → CB → Bulkhead → Timeout); each layer surfaces its own error type via `StackError`.

## Testing & tooling
- Run all tests: `cargo test` (unit, integration, doctests).  
- Deterministic time: circuit breaker accepts custom `Clock`; retries can swap sleepers (`InstantSleeper`, `TrackingSleeper`).  
- Decorrelated jitter tests include concurrent fuzzing for atomic state.  
- CI: actionlint → fmt → clippy → tests; release-plz config included.
- Docs: `cargo doc --no-deps` (also run in CI).

## Appendix A: Usage snippets
- Retry only: `RetryPolicy::builder().max_attempts(3).backoff(Backoff::linear(50.ms())).with_jitter(Jitter::equal())`.
- Timeout override: `TimeoutPolicy::new_with_max(Duration::from_secs(90), Duration::from_secs(3600))?`.
- Decorrelated jitter: `Jitter::decorrelated(100.ms(), 5.s())?.apply_stateful();`.
- Bulkhead unlimited: `BulkheadPolicy::unlimited()`.
- Default stack: `ResilienceStack::<Error>::default()` builds with sensible defaults.

## Appendix B: Test case definitions (canonical template)
Use this structure to trace tests to requirements:

- **TC ID**: e.g., `TC-BACKOFF-001`
- **Requirement**: e.g., “Backoff delay is 0 at attempt 0.”
- **Preconditions**: setup (strategy/base/max, runtime if needed).
- **Steps**: numbered actions.
- **Input Data**: specific durations/attempt counts.
- **Expected Results**: precise delays/states/errors.
- **Postconditions**: restored/default state.
- **Actual Results**: filled during execution.

Illustrative mappings (concise):
- `TC-BACKOFF-001` → attempt 0 yields 0 (tests: `delay_handles_zero_attempt`).
- `TC-BACKOFF-002` → monotonic + cap (tests: `delays_monotonic_and_capped`).
- `TC-JITTER-DECOR-UB` → decorrelated within [base..min(max,3*prev)] and non-regressing (tests: `decorrelated_respects_upper_bound_factor`, concurrency test).
- `TC-RETRY-STOP-NONINNER` → non-Inner errors not retried (tests: `test_resilience_error_not_retried`).
- `TC-RETRY-SHOULD-PRED` → predicate false short-circuits (tests: `should_retry_false_short_circuits`).
- `TC-TIMEOUT-MAX` → `new_with_max` obeys custom ceiling (tests: `new_with_max_respects_custom_boundaries`).
- `TC-STACK-DEFAULT` → default stack executes happy-path (tests: `default_stack_executes`).

## License
Apache-2.0
