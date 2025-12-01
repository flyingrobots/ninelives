# Nine Lives 🐱

> **Tower-native fractal supervision for async Rust.**
>
> Autonomous, self-healing Services via composable policy algebra.

[![Crates.io](https://img.shields.io/crates/v/ninelives.svg)](https://crates.io/crates/ninelives)
[![Documentation](https://docs.rs/ninelives/badge.svg)](https://docs.rs/ninelives)
[![License](https://img.shields.io/crates/l/ninelives.svg)](LICENSE)

**Nine Lives** is a resilience framework for Rust that treats failure handling as a composition problem. It provides standard patterns (retry, circuit breaker, bulkhead, timeout) as **Tower layers**, but supercharges them with an **algebraic composition system** that lets you express complex recovery strategies declaratively.

---

## 🚀 Quick Start (5 min)

1) Install & bootstrap (ensures fmt/clippy/hooks):

```bash
./scripts/bootstrap.sh
```

1) Add dependency (control enables the runtime command plane; schema-validation is on by default):

```toml
[dependencies]
ninelives = { version = "0.3", features = ["control"] }
tower = "0.5.2"
tokio = { version = "1", features = ["full"] }
```

1) Smoke test:

```bash
cargo test --all-features --all-targets
```

1) Minimal policy usage:

```rust
use ninelives::prelude::*;
use std::time::Duration;
use tower::{ServiceBuilder, ServiceExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let retry = simple::retry(3)?;              // default backoff/jitter
    let timeout = simple::timeout(Duration::from_secs(1))?;
    let policy = Policy(timeout) + Policy(retry);

    let mut svc = ServiceBuilder::new()
        .layer(policy)
        .service_fn(|req: &str| async move { Ok::<_, std::io::Error>(format!("echo: {req}")) });

    let out = svc.ready().await?.call("hi").await?;
    println!("{out}");
    Ok(())
}
```

1) Control-plane health & error shape (JSON, default schema validation enabled):

```json
// Health request (via your transport)
{ "id":"cmd-1", "cmd":"health", "args":{}, "auth": null }

// Error response example (structured CommandFailure)
{ "result":"error",
  "kind":{"kind":"invalid_args","msg":"missing key"},
  "message":"missing key" }
```

**Bootstrap shortcut (dev defaults; replace PassthroughAuth for production):**

```rust
use std::sync::Arc;
use ninelives::control::{bootstrap_defaults, BuiltInHandler};
use ninelives::control::BuiltInCommand;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let handler = Arc::new(BuiltInHandler::default());
    let (_router, transport) = bootstrap_defaults(handler);

    // Send a health check via the in-process ChannelTransport
    let cmd = ninelives::control::CommandEnvelope {
        cmd: BuiltInCommand::Health,
        auth: None,
        meta: ninelives::control::CommandMeta { id: "health-1".into(), correlation_id: None, timestamp_millis: None },
    };
    let res = transport.send(cmd).await?;
    println!("Health response: {:?}", res);
    Ok(0)
}
```

For more on payloads and validation see `docs/CONTROL_PLANE_SCHEMA.md`. Schema validation is **on by default** and can be disabled at runtime with `NINELIVES_SCHEMA_VALIDATION=0|false`; see `docs/SCHEMA_VALIDATION.md` for details.

**Circuit Breaker Registry semantics:** IDs must be unique. If the same ID is registered twice, the last registration replaces the prior handle and a warning is logged. Prefer distinct IDs per breaker to avoid accidental replacement.

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

## 📦 Cargo Features

* **`control`**: Enables the Control Plane (schema-validated JSON transport, command router, auth). *Default: enabled in full build, opt-in.*
* **`adaptive-rwlock`**: Switches the `Adaptive<T>` configuration backend to use `RwLock` (stronger consistency) instead of the default lock-free `ArcSwap` (higher performance).
* **`arc-swap`**: (Default) Enables the lock-free configuration backend.

---

## 🎛️ Control Plane

Turn static configs into live knobs. Nine Lives includes a runtime configuration system that lets you adjust max retries, timeouts, or circuit breaker thresholds without restarting the service.

### Wire Format & Schema

The Control Plane uses a canonical JSON envelope. Schema validation is **enabled by default** via `jsonschema`.

**Request Envelope:**

```json
{
  "id": "req-123",
  "cmd": "write_config",
  "args": { "path": "retry.max_attempts", "value": "5" },
  "auth": {
    "Jwt": { "token": "eyJhbG..." }
  }
}
```

*Supported AuthPayloads:* `Jwt`, `Signatures`, `Mtls`, `Opaque`.

**Response (Success):**

```json
{ "result": "ack", "id": "req-123" }
```

**Response (Error):**
Errors return a structured `CommandFailure` object.

```json
{
  "result": "error",
  "id": "req-123",
  "message": "unknown config path: foo",
  "kind": { "kind": "invalid_args", "msg": "unknown config path: foo" }
}
```

**⚠️ Persistence Warning:**
The `ConfigRegistry` is in-memory only. Configuration changes are volatile and will be lost on restart. To persist changes, you must implement the "Snapshot & Restore" pattern using the `GetState` command (export) and `apply_snapshot` API (import). See [docs/ADR-012-config-persistence.md](docs/ADR-012-config-persistence.md).

For full details, see [docs/control-plane.md](docs/control-plane.md).

### 🎯 What to read next

* Payload contracts and schemas: `docs/CONTROL_PLANE_SCHEMA.md`
* Operations (health, validation defaults, snapshot/restore): `docs/OPERATIONS.md`
* Persistence stance and snapshot hook: `docs/ADR-012-config-persistence.md`

---

## 🔌 Ecosystem

Nine Lives is designed to integrate with your infrastructure:

* [`ninelives-elastic`](ninelives-elastic/README.md)
* [`ninelives-etcd`](ninelives-etcd/README.md)
* [`ninelives-jsonl`](ninelives-jsonl/README.md)
* [`ninelives-kafka`](ninelives-kafka/README.md)
* [`ninelives-nats`](ninelives-nats/README.md)
* [`ninelives-otlp`](ninelives-otlp/README.md)
* [`ninelives-prometheus`](ninelives-prometheus/README.md)

---

## 🆚 Comparison

| Feature                               | Nine Lives | Resilience4j (Java) | Polly (C#) | go-kit (Go) | `tower` (Rust) |
| :------------------------------------ | :-----------: | :-----------------: | :--------: | :---------: | :------------: |
| **1. Uniform `Service` Abstraction**  |       ✅       |          ❌          |     ❌      |      ✅      |       ✅        |
| **2. Fractal/Recursive Architecture** |       ✅       |          ❌          |     ❌      |      ❌      |       ✅        |
| **3. Algebraic Composition**          |      Yes       |         No          |     No      |      No      |       No        |
| **4. Composable Telemetry Sinks**     |       ✅       |          ❌          |     ❌      |      ❌      |       ❌        |
| **5. Live Policy Updates**            |       ✅       |          ✅          |     ✅      |   Partial   |       ❌        |
| **6. Pluggable Control Plane**        |       ✅       |          ❌          |     ❌      |      ❌      |       ❌        |
| **7. Autonomous Self-Healing Loop**   |       ✅       |          ❌          |     ❌      |      ❌      |       ❌        |
| **8. Distributed/Fleet Policies**     |       ✅       |          ❌          |  Partial   |      ❌      |       ❌        |
| **9. Lock-Free Core**                 |       ✅       |          ⚠️          |     ⚠️      |      ⚠️      |       ⚠️        |

---

## 🗺️ Roadmap

* **Phase 1: Foundation** ✅ (Layers, Algebra, Telemetry)
* **Phase 2: Control Plane** ✅ (Runtime Config, Command Protocol)
* **Phase 3: Observer** 🚧 (Aggregation, Sentinel Logic)
* **Future:** WASM-based Meta-Policies, Distributed Circuit Breaking.

See [docs/ROADMAP/README.md](docs/ROADMAP/README.md) for details.

---

## 📄 License

Licensed under the Apache License, Version 2.0 (see [LICENSE](LICENSE)).

---
<div align="center">
  <b>Built with ❤️ for the Rust async ecosystem.</b>
</div>

© 2025 James Ross • [Flying Robots](https://github.com/flyingrobots)
