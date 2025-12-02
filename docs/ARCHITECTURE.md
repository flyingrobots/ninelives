# Nine Lives Architecture

Nine Lives is designed as a layered system for building resilient asynchronous applications.

## 1. Core Algebra & Layers

At the heart of the library are **Tower Layers** representing resilience primitives (`Retry`, `CircuitBreaker`, `Bulkhead`, `Timeout`).

The **Algebra** (`src/algebra.rs`) allows these layers to be composed declaratively:

* `+` (Wrap): Sequential composition.
* `|` (Fallback): Try A, then B.
* `&` (Race): Fork-join concurrency.

This allows complex policies (e.g., "Retry fast path, fallback to slow path with circuit breaker") to be expressed as `policy = (fast + retry) | (slow + breaker)`.

## 2. Telemetry

The telemetry system (`src/telemetry.rs`) decouples policy execution from observability.

* **PolicyEvent**: Structured enum describing what happened (e.g., `RetryAttempt`, `CircuitOpened`).
* **TelemetrySink**: A `tower::Service<PolicyEvent>` that consumes these events.
* **Composability**: Sinks can be multicast or fallbacked just like policies.

## 3. Control Plane

The Control Plane (`src/control/`) enables runtime inspection and reconfiguration of the application without redeployment.

### Data Flow

`Transport` -> `TransportEnvelope` -> `AuthRegistry` -> `CommandRouter` -> `BuiltInHandler` -> `ConfigRegistry` / `BreakerRegistry`.

* **Transport**: Decodes raw bytes (JSON, Protobuf) into a canonical `CommandEnvelope`.
* **AuthRegistry**: Verifies the `AuthPayload` (JWT, mTLS) attached to the envelope.
* **CommandRouter**: Dispatches the command to the appropriate handler and records the result in `CommandHistory` and `AuditSink`.
* **ConfigRegistry**: Holds `Adaptive<T>` handles. When a `WriteConfig` command is received, the registry updates the shared `Adaptive` value, which is immediately visible to the application's hot path.

### Dynamic Configuration (`Adaptive<T>`)

Configuration values (e.g., timeouts, retry counts) are wrapped in `Adaptive<T>`.

* **Lock-Free (Default)**: Uses `arc-swap` for high-performance reads on the hot path.
* **Strong Consistency (`adaptive-rwlock`)**: Optional feature to use `RwLock` for strict serialization.

### Persistence

The Control Plane is **in-memory only**. To persist changes:

1. **Export**: Use `GetState` to retrieve the current config snapshot.
2. **Store**: Save this snapshot to an external store (File, DB, K8s ConfigMap).
3. **Restore**: On startup, load the snapshot and use the `apply_snapshot` API (via `ConfigRegistry`) to re-hydrate the state.
