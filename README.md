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

* **`Policy(A) + Policy(B)` (Wrap):** `A` wraps `B` to create a sequential pipeline. This is standard Tower layering, where the outer layer `A` processes requests before passing them to the inner layer `B`.
  * *Example:* `Retry + Timeout` implies a Timeout mechanism applied *after* an operation has potentially retried.

* **`Policy(A) | Policy(B)` (Fallback):** Tries `A` first. If `A` fails, `B` is then attempted with the original request. This enables graceful degradation.
  * *Example:* `FastCache | SlowDatabase` will try to fetch from a fast cache, and only if that fails, query a slower database.

* **`Policy(A) & Policy(B)` (Race):** Runs `A` and `B` concurrently. The first successful response from either `A` or `B` is returned. If both fail, an error is returned. This is useful for "Happy Eyeballs" patterns or redundant requests.
  * *Example:* `RegionA & RegionB` to race requests to two different regions, using the quicker response.

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
ninelives = "0.3"
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

Pick a recipe from [`ninelives-cookbook`](https://docs.rs/ninelives-cookbook/latest/ninelives_cookbook/):

- **Simple retry:** [`retry_fast`](https://docs.rs/ninelives-cookbook/latest/ninelives_cookbook/fn.retry_fast.html) — 3 attempts, 50ms exp backoff + jitter.
- **Latency guard:** [`timeout_p95`](https://docs.rs/ninelives-cookbook/latest/ninelives_cookbook/fn.timeout_p95.html) — 300ms budget.
- **Bulkhead:** [`bulkhead_isolate`](https://docs.rs/ninelives-cookbook/latest/ninelives_cookbook/fn.bulkhead_isolate.html) — protect shared deps.
- **API guardrail (intermediate):** [`api_guardrail`](https://docs.rs/ninelives-cookbook/latest/ninelives_cookbook/fn.api_guardrail.html) — timeout + breaker + bulkhead.
- **Reliable read (advanced):** [`reliable_read`](https://docs.rs/ninelives-cookbook/latest/ninelives_cookbook/fn.reliable_read.html) — fast path then fallback stack.
- **Hedged read (tricky):** [`hedged_read`](https://docs.rs/ninelives-cookbook/latest/ninelives_cookbook/fn.hedged_read.html) — fork-join two differently tuned stacks.
- **Hedge + fallback (god tier):** [`hedged_then_fallback`](https://docs.rs/ninelives-cookbook/latest/ninelives_cookbook/fn.hedged_then_fallback.html) — race two fast paths, then fall back to a sturdy stack.
- **Sensible defaults:** [`sensible_defaults`](https://docs.rs/ninelives-cookbook/latest/ninelives_cookbook/fn.sensible_defaults.html) — timeout + retry + bulkhead starter pack.

Most recipes are adaptive: retry/timeout/circuit/bulkhead knobs can be updated live via the `Adaptive<T>` handles.

See [`ninelives-cookbook/examples/`](ninelives-cookbook/examples) for runnable demos.

---

## 🧭 Repo Layout (workspace)

- `src/` — core policies, algebra, control plane.
- `schemas/` — JSON Schemas for control-plane envelopes/results.
- `ninelives-*` — integration crates (nats, kafka, elastic, etcd, prometheus, otlp, jsonl).
- `ninelives-cookbook/` — ready-made policy recipes + examples.
- `xtask/` — dev automation and integration test runners (`xtask it-*`).

## Appendix: Environment Variables

All project/test environment variables are prefixed with `NINE_LIVES_`.

| Name | Purpose | Used by | Default / Example |
| --- | --- | --- | --- |
| `NINE_LIVES_TEST_NATS_URL` | NATS endpoint for integration tests | `ninelives-nats` tests, `xtask it-nats` | `nats://127.0.0.1:4222` |
| `NINE_LIVES_TEST_KAFKA_BROKERS` | Kafka bootstrap list for integration tests | `ninelives-kafka` tests, `xtask it-kafka` | `127.0.0.1:9092` |
| `NINE_LIVES_TEST_ETCD_ENDPOINT` | etcd HTTP endpoint for integration tests | `ninelives-etcd` tests, `xtask it-etcd` | `http://127.0.0.1:2379` |
| `NINE_LIVES_TEST_ELASTIC_URL` | Elasticsearch URL for integration tests | `ninelives-elastic` tests, `xtask it-elastic` | `http://127.0.0.1:9200` |
| `NINE_LIVES_TEST_OTLP_ENDPOINT` | OTLP HTTP endpoint for integration tests | `ninelives-otlp` tests, `xtask it-otlp` | `http://127.0.0.1:4318` |

---

## 🔋 Features

### 🛡️ Standard Primitives
- **Retry:** Exponential/Linear/Constant backoff with full jitter support.
- **Circuit Breaker:** Lock-free implementation. Automatically opens on failure spikes to protect downstream.
- **Bulkhead:** Semaphored concurrency limits to prevent resource exhaustion.
- **Timeout:** Strict latency bounds.

### 🎛️ Control Plane (Adaptive)
Turn static configs into live knobs. Nine Lives includes a runtime configuration system (`ConfigRegistry`, `CommandRouter`) that lets you adjust max retries, timeouts, or circuit breaker thresholds without restarting the service.

### 🛰️ Control Plane Wire Format

- Canonical wire envelope: `TransportEnvelope { id, cmd, args, auth }`
- Auth payload matches the Rust enum shape, e.g.:
  ```json
  {
    "id": "cmd-1",
    "cmd": "write_config",
    "args": { "path": "max_attempts", "value": "5" },
    "auth": { "Jwt": { "token": "your-jwt" } }
  }
  ```
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
| **3. Algebraic Composition**      |          ✅          |     ❌      |      ❌      |       ❌        | ❌   |
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
