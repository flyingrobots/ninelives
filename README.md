# Nine Lives 🐱

> Tower-native fractal supervision for async Rust — autonomous, self-healing Services via composable policy algebra.

Resilience patterns for Rust with algebraic composition.

[![Crates.io](https://img.shields.io/crates/v/ninelives.svg)](https://crates.io/crates/ninelives)
[![Documentation](https://docs.rs/ninelives/badge.svg)](https://docs.rs/ninelives)
[![License](https://img.shields.io/crates/l/ninelives.svg)](LICENSE)

Nine Lives provides battle-tested resilience patterns (retry, circuit breaker, bulkhead, timeout) as composable [tower](https://github.com/tower-rs/tower) layers with a unique algebraic composition system.

## Features

- 🔁 **Retry policies** with exponential/linear/constant backoff and jitter
- ⚡ **Circuit breakers** with half-open state recovery
- 🚧 **Bulkheads** for concurrency limiting and resource isolation
- ⏱️ **Timeout policies** integrated with tokio
- 🧮 **Algebraic composition** via intuitive operators (`+`, `|`, `&`)
- 🏎️ **Fork-join** for concurrent racing (Happy Eyeballs pattern)
- 🔒 **Lock-free implementations** using atomics
- 🏗️ **Tower-native** - works with any tower `Service`
- 🌐 **Companion sinks** (OTLP, NATS, Kafka, Elastic, etcd, Prometheus, JSONL) via optional crates

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
ninelives = "0.1"
tower = "0.5"
tokio = { version = "1", features = ["full"] }
```

### Basic Usage

```rust
use ninelives::prelude::*;
use std::time::Duration;
use tower::{Service, ServiceBuilder, ServiceExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Apply a timeout to any service
    let mut svc = ServiceBuilder::new()
        .layer(TimeoutLayer::new(Duration::from_secs(1))?)
        .service_fn(|req: &str| async move {
            Ok::<_, std::io::Error>(format!("Response: {}", req))
        });

    let response = svc.ready().await?.call("hello").await?;
    println!("{}", response);
    Ok(())
}
```

## Algebraic Composition - The Nine Lives Advantage

Compose resilience strategies using intuitive operators:

- **`Policy(A) + Policy(B)`** - Sequential composition: `A` wraps `B`
- **`Policy(A) | Policy(B)`** - Fallback: try `A`, fall back to `B` on error
- **`Policy(A) & Policy(B)`** - Fork-join: try both concurrently, return first success

**Precedence:** `&` > `+` > `|` (like `*` > `+` > bitwise-or in math)

### Example: Fallback Strategy

Try an aggressive timeout first, fall back to a longer timeout on failure:

```rust
use ninelives::prelude::*;
use std::time::Duration;
use tower::{ServiceBuilder, Layer};

let fast = Policy(TimeoutLayer::new(Duration::from_millis(100))?);
let slow = Policy(TimeoutLayer::new(Duration::from_secs(5))?);
let policy = fast | slow;

let svc = ServiceBuilder::new()
    .layer(policy)
    .service_fn(|req| async { Ok::<_, std::io::Error>(req) });
```

### Example: Fork-Join (Happy Eyeballs)

Race two strategies concurrently and return the first success:

```rust
use ninelives::prelude::*;
use std::time::Duration;

// Create two timeout policies with different durations
let ipv4 = Policy(TimeoutLayer::new(Duration::from_millis(100))?);
let ipv6 = Policy(TimeoutLayer::new(Duration::from_millis(150))?);

// Race them concurrently - first success wins
let policy = ipv4 & ipv6;

let svc = ServiceBuilder::new()
    .layer(policy)
    .service_fn(|req| async { Ok::<_, std::io::Error>(req) });
```

### Example: Multi-Tier Resilience

Combine multiple strategies with automatic precedence:

```rust
use ninelives::prelude::*;
use std::time::Duration;

// Aggressive: just a fast timeout
let aggressive = Policy(TimeoutLayer::new(Duration::from_millis(50))?);

// Defensive: nested timeouts for retries
let defensive = Policy(TimeoutLayer::new(Duration::from_secs(10))?)
              + Policy(TimeoutLayer::new(Duration::from_secs(5))?);

// Try aggressive first, fall back to defensive
let policy = aggressive | defensive;
// Parsed as: Policy(Timeout50ms) | (Policy(Timeout10s) + Policy(Timeout5s))
```

### Example: Circuit Breaker with Retry

```rust
use ninelives::prelude::*;
use std::time::Duration;

// Build a retry policy with exponential backoff
let retry = RetryPolicy::builder()
    .max_attempts(3)
    .backoff(Backoff::exponential(Duration::from_millis(100)))
    .with_jitter(Jitter::full())
    .build()?;

// Configure circuit breaker
let circuit_breaker = CircuitBreakerLayer::new(
    CircuitBreakerConfig::default()
        .failure_threshold(5)
        .timeout_duration(Duration::from_secs(10))
)?;

// Compose: circuit breaker wraps retry
let policy = Policy(circuit_breaker) + Policy(retry.into_layer());
```

## Telemetry Sink Ladder

- **Baby mode:** `MemorySink::with_capacity(1_000)` for local inspection.
- **Intermediate:** `NonBlockingSink(LogSink)` to keep request paths non-blocking while logging.
- **Advanced:** `NonBlockingSink(OtlpSink)` + `StreamingSink` fan-out for in-cluster consumers.
- **GOD MODE:** `StreamingSink` → NATS/Kafka/Elastic via companion crates, with Observer + Sentinel auto-tuning when drop/evict metrics spike.

See recipes in `src/cookbook.rs` and companion cookbooks:
- `ninelives-otlp/README.md`
- `ninelives-nats/README.md`
- `ninelives-kafka/README.md`
- `ninelives-elastic/README.md`
- `ninelives-etcd/README.md`
- `ninelives-prometheus/README.md`
- `ninelives-jsonl/README.md`

## Cookbook (pick your recipe)

- **Simple retry:** `retry_fast` — 3 attempts, 50ms exp backoff + jitter.
- **Latency guard:** `timeout_p95` — 300ms budget.
- **Bulkhead:** `bulkhead_isolate(max)` — protect shared deps.
- **API guardrail (intermediate):** `api_guardrail` — timeout + breaker + bulkhead.
- **Reliable read (advanced):** `reliable_read` — fast path then fallback stack.
- **Hedged read (tricky):** `hedged_read` — fork-join two differently-tuned stacks.
- **Hedge + fallback (god tier):** `hedged_then_fallback` — race two fast paths, then fall back to a sturdy stack.
- **Sensible defaults:** `sensible_defaults` — timeout + retry + bulkhead starter pack.

Most recipes are adaptive: retry/timeout/circuit/bulkhead knobs can be updated live via the `Adaptive<T>` handles (see cookbook for details).

All live in `src/cookbook.rs`.
Moved to the `ninelives-cookbook` crate (see its README/examples).

## Tower Integration

Nine Lives layers work seamlessly with tower's `ServiceBuilder`:

```rust
use ninelives::prelude::*;
use tower::ServiceBuilder;
use std::time::Duration;

let service = ServiceBuilder::new()
    .layer(TimeoutLayer::new(Duration::from_secs(30))?)
    .layer(CircuitBreakerLayer::new(CircuitBreakerConfig::default())?)
    .layer(BulkheadLayer::new(10)?)
    .service(my_inner_service);
```

Or use the algebraic syntax:

```rust
let policy = Policy(TimeoutLayer::new(Duration::from_secs(30))?)
           + Policy(CircuitBreakerLayer::new(CircuitBreakerConfig::default())?)
           + Policy(BulkheadLayer::new(10)?);

let service = ServiceBuilder::new()
    .layer(policy)
    .service(my_inner_service);
```

## Available Layers

### TimeoutLayer

Enforces time limits on operations:

```rust
use ninelives::prelude::*;
use std::time::Duration;

let timeout = TimeoutLayer::new(Duration::from_secs(5))?;
```

### RetryLayer

Retries failed operations with configurable backoff and jitter:

```rust
use ninelives::prelude::*;
use std::time::Duration;

let retry = RetryPolicy::builder()
    .max_attempts(3)
    .backoff(Backoff::exponential(Duration::from_millis(100)))
    .with_jitter(Jitter::full())
    .build()?
    .into_layer();
```

**Backoff strategies:**
- `Backoff::constant(duration)` - Fixed delay
- `Backoff::linear(base)` - Linear increase: `base * attempt`
- `Backoff::exponential(base)` - Exponential: `base * 2^attempt`

**Jitter strategies:**
- `Jitter::none()` - No jitter
- `Jitter::full()` - Random [0, delay]
- `Jitter::equal()` - delay/2 + random [0, delay/2]
- `Jitter::decorrelated()` - AWS-style stateful jitter

### CircuitBreakerLayer

Prevents cascading failures with three-state management (Closed/Open/HalfOpen):

```rust
use ninelives::prelude::*;
use std::time::Duration;

let circuit_breaker = CircuitBreakerLayer::new(
    CircuitBreakerConfig::default()
        .failure_threshold(5)        // Open after 5 failures
        .timeout_duration(Duration::from_secs(10))  // Stay open for 10s
        .half_open_max_calls(3)      // Allow 3 test calls in half-open
)?;
```

### BulkheadLayer

Limits concurrent requests for resource isolation:

```rust
use ninelives::prelude::*;

let bulkhead = BulkheadLayer::new(10)?;  // Max 10 concurrent requests
```

## Error Handling

All resilience errors are unified under `ResilienceError<E>`:

```rust
use ninelives::ResilienceError;

match service.call(request).await {
    Ok(response) => { /* success */ },
    Err(ResilienceError::Timeout { .. }) => { /* timeout */ },
    Err(ResilienceError::CircuitOpen { .. }) => { /* circuit breaker open */ },
    Err(ResilienceError::RetryExhausted { failures, .. }) => {
        // All retry attempts failed
        eprintln!("Failed after {} attempts", failures.len());
    },
    Err(ResilienceError::Bulkhead { .. }) => { /* capacity exhausted */ },
    Err(ResilienceError::Inner(e)) => { /* inner service error */ },
}
```

## Operator Precedence

When combining operators, understand the precedence rules:

```rust
// & binds tighter than +, and + binds tighter than |
A | B + C & D   // Parsed as: A | (B + (C & D))

// Use parentheses for explicit control
(A | B) + C     // C wraps the fallback between A and B
```

**Examples:**

```rust
// Try fast, fallback to slow with retry
let policy = fast | retry + slow;
// Equivalent to: fast | (retry + slow)

// Retry wraps a fallback
let policy = retry + (fast | slow);

// Happy Eyeballs: race IPv4 and IPv6
let policy = ipv4 & ipv6;
// Both called concurrently, first success wins

// Complex composition
let policy = aggressive | defensive + (ipv4 & ipv6);
// Try aggressive, fallback to defensive wrapping parallel attempts
```

## Testability

Nine Lives is designed for testing with dependency injection:

```rust
use ninelives::prelude::*;
use std::time::Duration;

// Use InstantSleeper for tests (no actual delays)
let retry = RetryPolicy::builder()
    .max_attempts(3)
    .backoff(Backoff::exponential(Duration::from_millis(100)))
    .with_sleeper(InstantSleeper)
    .build()?;

// TrackingSleeper records sleep durations for assertions
let tracker = TrackingSleeper::new();
let retry = RetryPolicy::builder()
    .max_attempts(3)
    .with_sleeper(tracker.clone())
    .build()?;

// ... exercise retry ...

let sleeps = tracker.get_sleeps();
assert_eq!(sleeps.len(), 2); // Slept twice before success
```

## Roadmap

Nine Lives is evolving toward a **fractal resilience framework** with autonomous operation:

- **v1.0** (Current Phase): Tower-native layers with algebraic composition ✅
- **v1.5**: Telemetry events, control plane for runtime tuning 🚧
- **v2.0**: Autonomous Sentinel with meta-policies, shadow evaluation 🔮
- **v3.0**: Rich adapter ecosystem (Redis, OTLP, Prometheus) 🌐

See [ROADMAP.md](ROADMAP.md) for the full vision.

## Performance

Nine Lives is built for production:

- **Lock-free** circuit breaker state transitions using atomics
- **Zero-allocation** backoff/jitter calculations with overflow protection
- **Minimal overhead** - resilience layers add < 1% latency in common cases

Benchmarks coming soon.

## Comparison to Other Libraries

| Feature | Nine Lives | Resilience4j (Java) | Polly (C#) | tower |
|---------|-----------|---------------------|-----------|-------|
| Uniform Service Abstraction | ✅ | ❌ | ❌ | ✅ |
| Algebraic Composition (`+`, `\|`, `&`) | ✅ | ❌ | ❌ | ❌ |
| Fork-Join (Happy Eyeballs) | ✅ | ❌ | ❌ | ❌ |
| Tower Integration | ✅ Native | N/A | N/A | ✅ Native |
| Lock-Free Implementations | ✅ | Partial | Partial | Varies |
| Retry with Backoff/Jitter | ✅ | ✅ | ✅ | ❌ |
| Circuit Breaker | ✅ | ✅ | ✅ | ❌ |
| Bulkhead | ✅ | ✅ | ✅ | ❌ |
| Timeout | ✅ | ✅ | ✅ | ✅ |

**Nine Lives' unique advantage:** Algebraic composition with fork-join support lets you express complex resilience strategies declaratively, including concurrent racing patterns like Happy Eyeballs, without nested builders or imperative code.

## Examples

See the [`examples/`](examples/) directory for runnable examples:

- `retry_only.rs` - Basic retry with backoff
- `decorrelated_jitter.rs` - AWS-style decorrelated jitter
- `algebra_composition.rs` - Complex algebraic composition patterns

Run with:

```bash
cargo run --example retry_only
```

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you shall be dual licensed as above, without any additional terms or conditions.

---

**Built with ❤️ for the Rust async ecosystem.**
