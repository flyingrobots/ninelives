# Nine Lives 🐱

Production-grade resilience patterns for async Rust: retries, circuit breakers, bulkheads, and timeouts that compose cleanly.

## Features
- Retry policy with constant/linear/exponential backoff and pluggable jitter
- Circuit breaker with half-open probing, configurable limits, and injectable clock for deterministic tests
- Bulkhead (semaphore) concurrency limits plus "unlimited" mode
- Timeout wrapper using `tokio::time::timeout`
- Resilience stack builder to compose the policies in one call
- Test-friendly helpers (`InstantSleeper`, `TrackingSleeper`, custom clocks)

## Quick Start
```rust
use ninelives::{Backoff, Jitter, RetryPolicy, ResilienceError};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), ResilienceError<std::io::Error>> {
    let policy = RetryPolicy::builder()
        .max_attempts(3)
        .backoff(Backoff::exponential(Duration::from_secs(1)))
        .with_jitter(Jitter::full())
        .build()
        .expect("retry policy configuration to be valid");

    let value = policy
        .execute(|| async {
            // your fallible async operation
            Ok::<_, ResilienceError<std::io::Error>>("hello")
        })
        .await?;

    println!("{}", value);
    Ok(())
}
```

### Composing everything
```rust
use ninelives::{Backoff, Jitter, ResilienceStack};
use std::time::Duration;

let stack = ResilienceStack::new()
    .timeout(Duration::from_secs(2)).expect("valid timeout")
    .bulkhead(50).expect("valid bulkhead")
    .circuit_breaker(5, Duration::from_secs(30)).expect("valid breaker")
    .retry(
        ninelives::RetryPolicy::builder()
            .max_attempts(4)
            .backoff(Backoff::exponential(Duration::from_millis(100)))
            .with_jitter(Jitter::equal())
            .build()
            .expect("valid retry policy"),
    )
    .build()
    .expect("valid stack");

let result = stack
    .execute(|| async {
        // fallible work here
        Ok::<_, ninelives::ResilienceError<std::io::Error>>(())
    })
    .await;
```

## Builder fallibility
- Constructors and builders that validate inputs return `Result<_, Error>` and avoid panicking on bad configuration.
- Handle configuration early with `?`/`expect` so CI and startup catch issues immediately.
- `ResilienceStackBuilder` surfaces which layer failed through `StackError` (timeout, bulkhead, circuit breaker, or retry).

## Testing
- Unit tests live next to the code for tight access to internals. Run them with:
  - `cargo test`
- The circuit breaker accepts a custom clock (`with_clock`) so you can simulate time without sleeping. Retry policies accept custom sleepers and RNGs for deterministic testing.

## Project Status
Early-stage but fully tested (see `cargo test`). APIs may evolve while keeping ergonomics front and center.
