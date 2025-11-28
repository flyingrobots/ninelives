# Nine Lives 🐱

> **Tower-native fractal supervision for async Rust.**
>
> Autonomous, self-healing Services via composable policy algebra.

[![Crates.io](https://img.shields.io/crates/v/ninelives.svg)](https://crates.io/crates/ninelives)
[![Documentation](https://docs.rs/ninelives/badge.svg)](https://docs.rs/ninelives)
[![License](https://img.shields.io/crates/l/ninelives.svg)](LICENSE)

**Nine Lives** is a resilience framework for Rust that treats failure handling as a composition problem. It provides standard patterns (retry, circuit breaker, bulkhead, timeout) as **Tower layers**, but supercharges them with an **algebraic composition system** that lets you express complex recovery strategies declaratively.

---

## ⚗️ The Algebra of Resilience

These operators are recursive: a composed `Policy` is just another `Policy`, allowing you to snap them together like Lego blocks into arbitrarily complex supervision trees.

Nine Lives introduces three intuitive operators to compose `Policy` layers:

*   **`Policy(A) + Policy(B)` (Wrap):** `A` wraps `B` to create a sequential pipeline. This is standard Tower layering, where the outer layer `A` processes requests before passing them to the inner layer `B`.
    *   *Example:* `Retry + Timeout` implies a Timeout mechanism applied *after* an operation has potentially retried.

*   **`Policy(A) | Policy(B)` (Fallback):** Tries `A` first. If `A` fails, `B` is then attempted with the original request. This enables graceful degradation.
    *   *Example:* `FastCache | SlowDatabase` will try to fetch from a fast cache, and only if that fails, query a slower database.

*   **`Policy(A) & Policy(B)` (Race):** Runs `A` and `B` concurrently. The first successful response from either `A` or `B` is returned. If both fail, an error is returned. This is useful for "Happy Eyeballs" patterns or redundant requests.
    *   *Example:* `RegionA & RegionB` to race requests to two different regions, using the quicker response.

### Expressive Composition

Combine strategies naturally with operator precedence (`&` > `+` > `|`). No more nested builder hell.

```rust
// "Try the fast path. If it fails, retry the slow path with a circuit breaker."
let strategy = fast_path | (retry + breaker + slow_path);
```

---

## 🚀 Quick Start

Add to `Cargo.toml`:
```toml
[dependencies]
ninelives = "latest"
tower = "0.5.2"
tokio = { version = "1", features = ["full"] }
```

### Basic Usage

```rust
use ninelives::prelude::*;
use std::time::Duration;
use tower::{ServiceBuilder, ServiceExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Define a policy: Retry 3 times with backoff, wrapped in a 1s timeout
    let retry = RetryPolicy::builder()
        .max_attempts(3)
        .backoff(Backoff::exponential(Duration::from_millis(50)))
        .build()?
        .into_layer();

    let policy = Policy(TimeoutLayer::new(Duration::from_secs(1))?) + Policy(retry);

    let svc = ServiceBuilder::new()
        .layer(policy)
        .service_fn(|req: &str| async move {
            // Your potentially failing logic here
            Ok::<_, std::io::Error>(format!("Echo: {}", req))
        });

    let response = svc.ready().await?.call("hello").await?;
    println!("{}", response);
    Ok(())
}
```

---

## 🍳 Cookbook

Pick a recipe from [`ninelives-cookbook`](ninelives-cookbook/src/lib.rs):

- **Simple retry:** [`retry_fast`](ninelives-cookbook/src/lib.rs#L15) — 3 attempts, 50ms exp backoff + jitter.
- **Latency guard:** [`timeout_p95`](ninelives-cookbook/src/lib.rs#L33) — 300ms budget.
- **Bulkhead:** [`bulkhead_isolate(max)`](ninelives-cookbook/src/lib.rs#L39) — protect shared deps.
- **API guardrail (intermediate):** [`api_guardrail`](ninelives-cookbook/src/lib.rs#L74) — timeout + breaker + bulkhead.
- **Reliable read (advanced):** [`reliable_read`](ninelives-cookbook/src/lib.rs#L48) — fast path then fallback stack.
- **Hedged read (tricky):** [`hedged_read`](ninelives-cookbook/src/lib.rs#L90) — fork-join two differently tuned stacks.
- **Hedge + fallback (god tier):** [`hedged_then_fallback`](ninelives-cookbook/src/lib.rs#L129) — race two fast paths, then fall back to a sturdy stack.
- **Sensible defaults:** [`sensible_defaults`](ninelives-cookbook/src/lib.rs#L112) — timeout + retry + bulkhead starter pack.

Most recipes are adaptive: retry/timeout/circuit/bulkhead knobs can be updated live via the `Adaptive<T>` handles.

See [`ninelives-cookbook/examples/`](ninelives-cookbook/examples) for runnable demos.

---

## 🔋 Features

### 🛡️ Standard Primitives
- **Retry:** Exponential/Linear/Constant backoff with full jitter support.
- **Circuit Breaker:** Lock-free implementation. Automatically opens on failure spikes to protect downstream.
- **Bulkhead:** Semaphored concurrency limits to prevent resource exhaustion.
- **Timeout:** Strict latency bounds.

### 🎛️ Control Plane (Adaptive)
Turn static configs into live knobs. Nine Lives includes a runtime configuration system (`ConfigRegistry`, `CommandRouter`) that lets you adjust max retries, timeouts, or circuit breaker thresholds without restarting the service.

### 📡 Telemetry & Observability
Unified event system. Every layer emits structured `PolicyEvent`s (e.g., `RetryAttempt`, `CircuitOpen`).
- **Sinks:**
  - `LogSink` (tracing)
  - `OtlpSink` (OpenTelemetry)
  - `StreamingSink` (Broadcast to NATS/Kafka)
- **Introspection:** Query the state of any circuit breaker at runtime.

### 🔌 Ecosystem
Nine Lives is designed to integrate with your infrastructure:
- [`ninelives-elastic`](ninelives-elastic/README.md)
- [`ninelives-etcd`](ninelives-etcd/README.md)
- [`ninelives-jsonl`](ninelives-jsonl/README.md)
- [`ninelives-kafka`](ninelives-kafka/README.md)
- [`ninelives-nats`](ninelives-nats/README.md)
- [`ninelives-otlp`](ninelives-otlp/README.md)
- [`ninelives-prometheus`](ninelives-prometheus/README.md)

---

## 🆚 Comparison

| Feature                               | Nine Lives | Resilience4j (Java) | Polly (C#) | go-kit (Go) | `tower` (Rust) |
| :------------------------------------ | :-----------: | :-----------------: | :--------: | :---------: | :------------: |
| **1. Uniform `Service` Abstraction**  |       ✅       |          ❌          |     ❌      |      ✅      |       ✅        |
| **2. Fractal/Recursive Architecture** |       ✅       |          ❌          |     ❌      |      ❌      |       ✅        |
| **3. Algebraic Composition** (`+`, `\|`, `&`)      |          ✅          |     ❌      |      ❌      |       ❌        | ❌   |
| **4. Composable Telemetry Sinks**     |       ✅       |          ❌          |     ❌      |      ❌      |       ❌        |
| **5. Live Policy Updates**            |       ✅       |          ✅          |     ✅      |   Partial   |       ❌        |
| **6. Pluggable Control Plane**        |       ✅       |          ❌          |     ❌      |      ❌      |       ❌        |
| **7. Autonomous Self-Healing Loop**   |       ✅       |          ❌          |     ❌      |      ❌      |       ❌        |
| **8. Distributed/Fleet Policies**     |       ✅       |          ❌          |  Partial   |      ❌      |       ❌        |
| **9. Lock-Free Core**                 |       ✅       |          ⚠️          |     ⚠️      |      ⚠️      |       ⚠️        |

---

## 🗺️ Roadmap

- **Phase 1: Foundation** ✅ (Layers, Algebra, Telemetry)
- **Phase 2: Control Plane** ✅ (Runtime Config, Command Protocol)
- **Phase 3: Observer** 🚧 (Aggregation, Sentinel Logic)
- **Future:** WASM-based Meta-Policies, Distributed Circuit Breaking.

See [ROADMAP.md](docs/ROADMAP/README.md) for details.

---

## 📄 License

Licensed under the Apache License, Version 2.0 (see [LICENSE](LICENSE)).

---
<div align="center">
  <b>Built with ❤️ for the Rust async ecosystem.</b>
</div>

© 2025 James Ross • [Flying Robots](https://github.com/flyingrobots)
