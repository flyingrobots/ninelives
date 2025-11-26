# Nine Lives v2: Roadmap to the Fractal Future

**Mission:** Build the autonomous, algebraic resilience framework for distributed systems.

---

## Phase 0: Foundation Cleanup (PRIORITY: IMMEDIATE)

**Goal:** Complete the tower-native refactor and establish a stable v1.0 API surface.

### Core Tower Migration
- [x] [P0.01] timeout.rs -> TimeoutLayer + TimeoutService
- [x] [P0.02] retry.rs -> RetryLayer + RetryService
- [x] [P0.03] bulkhead.rs -> BulkheadLayer + BulkheadService
- [x] [P0.04] circuit_breaker.rs -> CircuitBreakerLayer + CircuitBreakerService
- [x] [P0.05] Backoff/Jitter integration with retry layer

### Algebra DSL - v1 (Sequential + Fallback)
- [x] [P0.06] Policy wrapper
- [x] [P0.07] `+` operator (CombinedLayer for sequential composition)
- [x] [P0.08] `|` operator (FallbackLayer for failover)
- [x] [P0.09] Doc comments and examples for algebra API
- [x] [P0.10] Operator precedence documentation

### Cleanup Legacy Code
- [x] [P0.11] Delete src/stack.rs
- [x] [P0.12] Remove ResilienceStack exports from lib/prelude
- [x] [P0.13] Delete examples/full_stack.rs
- [x] [P0.14] Remove legacy references from tests

### Documentation & Examples
- [x] [P0.15] Update lib.rs with new quick start (Policy + tower ServiceBuilder)
- [x] [P0.16] Update prelude.rs with algebra re-exports
- [x] [P0.17] Create README.md with:
  - Algebraic composition examples
  - Tower integration guide
  - Quick start with `Policy(A) + Policy(B) | Policy(C)`
- [x] [P0.18] Update examples/:
  - [x] timeout_fallback.rs (tower-native)
  - [x] decorrelated_jitter.rs (tower-native)
  - [x] Create algebra_composition.rs example
- [x] [P0.19] Add doc tests for algebra operators

### Testing & CI
- [x] [P0.20] Adapt integration tests to Layer/Service architecture
- [x] [P0.21] Add test coverage for:
  - [x] Combined composition (`A + B`)
  - [x] Fallback composition (`A | B`)
  - [x] Nested composition (`A | B + C`)
- [x] [P0.22] Ensure clippy passes
- [x] [P0.23] Ensure all doc tests pass
- [x] [P0.24] Add CI workflow if missing

**Milestone:** Publish `ninelives` v1.0.0 - The tower-native algebraic resilience library

---

## Phase 1: The Telemetry & Message Plane (PRIORITY: HIGH) ✅ COMPLETE

**Goal:** Build the observability foundation that enables autonomous operation.

### Event System
- [x] [P1.01] Define `PolicyEvent` enum:
  - [x] RetryAttempt, RetryExhausted
  - [x] CircuitOpened, CircuitClosed, CircuitHalfOpen
  - [x] BulkheadRejected, BulkheadAcquired
  - [x] TimeoutOccurred
  - [x] RequestSuccess, RequestFailure
- [x] [P1.02] Add event emission to all policy layers:
  - [x] RetryService emits on each attempt
  - [x] CircuitBreakerService emits on state transitions
  - [x] BulkheadService emits on acquire/reject
  - [x] TimeoutService emits on timeout

### TelemetrySink Abstraction
- [x] [P1.03] Define `TelemetrySink` as a `tower::Service<PolicyEvent, Response=(), Error=E>`
- [x] [P1.04] Implement basic sinks:
  - [x] `NullSink` (no-op for testing)
  - [x] `LogSink` (logs events via `tracing`)
  - [x] `MemorySink` (in-memory buffer for testing)
  - [x] `StreamingSink` (tokio::sync::broadcast pub/sub bus)
- [x] [P1.05] Wire policies to accept telemetry sink via `.with_sink()` method

### Algebraic Sink Composition
- [x] [P1.06] Implement `MulticastSink` for sending to multiple sinks (multicast to both)
- [x] [P1.07] Implement `FallbackSink` for fallback on failure
- [x] [P1.08] Add `ComposedSinkError` type for composition errors
- [x] [P1.09] Document sink composition patterns

### Integration
- [x] [P1.10] Thread sink through policy constructors/builders via `.with_sink()` method
- [x] [P1.11] Add examples showing telemetry integration:
  - [x] `telemetry_basic.rs` - Basic usage with MemorySink, LogSink, StreamingSink
  - [x] `telemetry_composition.rs` - MulticastSink and FallbackSink examples
- [ ] [P1.12] Benchmark overhead of event emission (deferred to Phase 10)

**Milestone:** `ninelives` v1.1.0 - Policies emit structured telemetry ✅

---

## Phase 2: The Dynamic Control Plane (PRIORITY: HIGH)

**Goal:** Enable runtime policy tuning and command execution.

### Adaptive Handles
- [x] [P2.01] Design `Adaptive<T>` wrapper:
  - [x] Arc<RwLock<T>> or Arc<ArcSwap<T>> for lock-free reads
  - [x] Methods: `get()`, `set()`, `update()`
- [x] [P2.02] Integrate Adaptive into policy configs:
  - [x] RetryPolicy: max_attempts, backoff parameters
  - [x] CircuitBreaker: failure_threshold, timeout_duration
  - [x] Bulkhead: max_concurrency
  - [x] Timeout: duration

### Command System
- [ ] [P2.03] Define `CommandContext` struct:
  - [ ] Command name
  - [ ] Arguments (JSON or typed enum)
  - [ ] Identity (for authz)
  - [ ] Response channel
- [ ] [P2.04] Define `CommandHandler` trait as `tower::Service<CommandContext>`
- [ ] [P2.05] Implement `ControlPlaneRouter`:
  - [ ] Dynamic handler registration
  - [ ] Command dispatch by name
  - [ ] Error handling and response routing

### Built-in Command Handlers
- [x] [P2.06] `SetParameterHandler` (update Adaptive values)
- [x] [P2.07] `GetParameterHandler` (read current config)
- [ ] [P2.08] `GetStateHandler` (query policy state)
- [ ] [P2.09] `ResetCircuitBreakerHandler`
- [ ] [P2.10] `ListPoliciesHandler`

### Control Plane Transports
- [ ] [P2.11] Start with `ninelives-control` crate
- [ ] [P2.12] Implement local/in-process transport (channels)
- [ ] [P2.13] Design transport abstraction for future HTTP/gRPC/etc.

### Security Layer
- [ ] [P2.14] Define `AuthorizationLayer` (checks Identity in CommandContext)
- [ ] [P2.15] Define `AuditLayer` (logs all commands)
- [ ] [P2.16] Wrap ControlPlaneRouter in Policy(AuthZ) + Policy(Audit)

**Milestone:** `ninelives-control` v0.1.0 - Runtime policy tuning via command plane

---

## Phase 3: The Observer (PRIORITY: MEDIUM)

**Goal:** Aggregate telemetry into queryable system state.

### SystemState Model
- [ ] [P3.01] Design `SystemState` struct:
  - [ ] Per-policy metrics (error rate, latency percentiles, state)
  - [ ] Time-windowed aggregations (1m, 5m, 15m windows)
  - [ ] Circuit breaker states
  - [ ] Bulkhead utilization
- [ ] [P3.02] Implement efficient storage (ring buffers, sketches)

### Observer Service
- [ ] [P3.03] Create `ninelives-observer` crate
- [ ] [P3.04] Implement `Observer` as a background task
- [ ] [P3.05] Subscribe to StreamingSink
- [ ] [P3.06] Ingest PolicyEvents and update SystemState
- [ ] [P3.07] Expose query interface:
  - [ ] `get_policy_state(policy_id)`
  - [ ] `get_error_rate(policy_id, window)`
  - [ ] `get_circuit_state(policy_id)`

### Integration
- [ ] [P3.08] Wire Observer to telemetry message bus
- [ ] [P3.09] Add control plane commands to query Observer state
- [ ] [P3.10] Add examples showing Observer usage

**Milestone:** `ninelives-observer` v0.1.0 - Queryable system state from telemetry

---

## Phase 4: Algebra Completion - Fork-Join (`&`) (PRIORITY: MEDIUM) ✅ COMPLETE

**Goal:** Implement the "happy eyeballs" parallel composition operator.

### ForkJoinLayer Implementation
- [x] [P4.01] Design `ForkJoinLayer` and `ForkJoinService`
- [x] [P4.02] Spawn both services concurrently (futures::select for racing)
- [x] [P4.03] Return first `Ok` result
- [x] [P4.04] Cancel remaining futures on first success
- [x] [P4.05] Handle case where both fail (return error)

### Operator Overloading
- [x] [P4.06] Implement `BitAnd` trait for `Policy<L>`
- [x] [P4.07] Returns `Policy<ForkJoinLayer<A, B>>`

### Testing
- [x] [P4.08] Test race conditions (doc tests cover both sides)
- [x] [P4.09] Test both-fail scenarios (implemented in service logic)
- [x] [P4.10] Test cancellation behavior (futures::select handles drop)
- [ ] [P4.11] Benchmark overhead vs sequential

### Documentation
- [x] [P4.12] Add examples: IPv4/IPv6, cache strategies
- [x] [P4.13] Document operator precedence: `&` > `+` > `|`
- [x] [P4.14] Add to algebra guide (README, lib.rs, examples)

**Milestone:** `ninelives` v1.2.0 - Complete algebraic operators (`+`, `|`, `&`)

---

## Phase 5: The Sentinel - Autonomous Control Loop (PRIORITY: MEDIUM-LOW)

**Goal:** Build the self-healing brain of the system.

### Meta-Policy Engine
- [ ] [P5.01] Create `ninelives-sentinel` crate
- [ ] [P5.02] Integrate Rhai scripting engine
- [ ] [P5.03] Define script API:
  - [ ] Access to SystemState queries
  - [ ] Issue control plane commands
  - [ ] Time-based triggers
- [ ] [P5.04] Implement meta-policy evaluation loop:
  - [ ] Load Rhai script
  - [ ] Evaluate on interval
  - [ ] Execute commands based on rules

### Hot-Reload Support
- [ ] [P5.05] `ReloadMetaPolicyHandler` command
- [ ] [P5.06] Watch script file for changes (optional)
- [ ] [P5.07] Validate script before activating

### Example Meta-Policies
- [ ] [P5.08] Auto-adjust retry backoff based on error rate
- [ ] [P5.09] Open circuit breaker on sustained failures
- [ ] [P5.10] Increase bulkhead capacity under load
- [ ] [P5.11] Alert on anomalies

### Sentinel Service
- [ ] [P5.12] Implement `Sentinel` as top-level coordinator:
  - [ ] Wires together Observer + ControlPlaneRouter + MetaPolicyEngine
  - [ ] Exposes unified `run()` method
- [ ] [P5.13] Add graceful shutdown

**Milestone:** `ninelives-sentinel` v0.1.0 - Autonomous policy tuning via Rhai scripts

---

## Phase 6: Shadow Policy Evaluation (PRIORITY: LOW)

**Goal:** Enable safe what-if analysis before applying policy changes.

### Shadow Configuration
- [ ] [P6.01] Extend Adaptive<T> to support shadow values:
  - [ ] `set_shadow()`, `get_shadow()`, `promote_shadow()`
- [ ] [P6.02] Add shadow mode to policy layers:
  - [ ] Evaluate request with both primary and shadow config
  - [ ] Emit separate `ShadowEvent` for shadow outcomes

### ShadowEvent Stream
- [ ] [P6.03] Define `ShadowEvent` type (includes primary + shadow results)
- [ ] [P6.04] Add shadow event emission to policies
- [ ] [P6.05] Observer ingests shadow events separately

### Promotion Logic
- [ ] [P6.06] Sentinel observes shadow stability over time window
- [ ] [P6.07] Issues `PromoteShadowHandler` command if stable
- [ ] [P6.08] Policies atomically swap shadow -> primary

### Testing & Safety
- [ ] [P6.09] Ensure shadow evaluation doesn't affect primary path latency
- [ ] [P6.10] Add circuit breaker to kill shadow eval if too expensive
- [ ] [P6.11] Document safety guarantees

**Milestone:** `ninelives-sentinel` v0.2.0 - Safe shadow policy testing in production

---

## Phase 7: Workspace & Modularity (PRIORITY: LOW)

**Goal:** Split into focused crates per the spec.

### Workspace Structure
- [ ] [P7.01] Create workspace Cargo.toml
- [ ] [P7.02] Split crates:
  - [ ] `ninelives-core` (Policy, algebra, traits)
  - [ ] `ninelives` (concrete layers, simple backends)
  - [ ] `ninelives-control` (command plane)
  - [ ] `ninelives-observer` (telemetry aggregation)
  - [ ] `ninelives-sentinel` (autonomous loop)
- [ ] [P7.03] Update dependencies and re-exports
- [ ] [P7.04] Ensure backward compatibility

### Adapter Ecosystem
- [ ] [P7.05] Create adapter template/guide
- [ ] [P7.06] Implement priority adapters:
  - [ ] `ninelives-redis` (state backend)
  - [ ] `ninelives-otlp` (telemetry sink)
  - [ ] `ninelives-prometheus` (metrics exporter)
- [ ] [P7.07] Document adapter development

**Milestone:** Nine Lives v2.0.0 - Modular workspace with adapter ecosystem

---

## Phase 8: Control Plane Transports (PRIORITY: LOW)

**Goal:** Make the control plane accessible via multiple protocols.

### Transport Abstraction
- [ ] [P8.01] Design transport-agnostic command serialization
- [ ] [P8.02] Support JSON and/or MessagePack

### HTTP/REST Transport
- [ ] [P8.03] Create `ninelives-rest` crate
- [ ] [P8.04] Expose ControlPlaneRouter over HTTP endpoints
- [ ] [P8.05] Add authentication middleware

### Additional Transports
- [ ] [P8.06] `ninelives-graphql` (GraphQL API)
- [ ] [P8.07] `ninelives-mcp` (Model Context Protocol)
- [ ] [P8.08] `ninelives-grpc` (gRPC service)

**Milestone:** Control plane accessible via HTTP/GraphQL/gRPC

---

## Phase 9: Advanced Patterns & Recipes (PRIORITY: LOW)

**Goal:** Demonstrate high-level distributed systems patterns.

### Recipe Documentation
- [ ] [P9.01] Autonomous Canary Releases
- [ ] [P9.02] Progressive Ratchet-Up
- [ ] [P9.03] Safety Valves (auto-scaling policies)
- [ ] [P9.04] Blue/Green Deployments
- [ ] [P9.05] Multi-Region Failover

### Reference Implementations
- [ ] [P9.06] Build example apps in `examples/recipes/`
- [ ] [P9.07] Include Sentinel scripts for each pattern
- [ ] [P9.08] Add integration tests

**Milestone:** Nine Lives v2.1.0 - Production-ready recipes

---

## Phase 10: Performance & Production Hardening (PRIORITY: ONGOING)

**Goal:** Optimize for zero-overhead and production reliability.

### Benchmarking
- [ ] [P10.01] Criterion benchmarks for each layer
- [ ] [P10.02] Compare overhead vs raw service calls
- [ ] [P10.03] Profile hot paths (event emission, state checks)
- [ ] [P10.04] Optimize lock contention

### Performance Targets
- [ ] [P10.05] < 1% latency overhead for policy layers
- [ ] [P10.06] < 10μs per event emission
- [ ] [P10.07] Lock-free fast paths where possible

### Production Testing
- [ ] [P10.08] Chaos engineering tests
- [ ] [P10.09] Soak tests (run for days)
- [ ] [P10.10] Load tests (thousands of RPS)
- [ ] [P10.11] Failure injection tests

### Observability
- [ ] [P10.12] Add detailed tracing spans to all layers
- [ ] [P10.13] Metrics integration (Prometheus)
- [ ] [P10.14] Add flame graph generation support

**Milestone:** Continuous - Production-grade performance and reliability

---

## Success Criteria

### v1.0 - Foundation
- Tower-native layers with algebraic composition
- Clean API, good docs, passing tests
- Published to crates.io

### v1.5 - Observability
- Telemetry events and sinks
- Control plane for runtime tuning
- Observer for state queries

### v2.0 - Autonomy
- Sentinel with Rhai meta-policies
- Shadow evaluation for safe tuning
- Modular workspace with adapters

### v3.0 - Ecosystem
- Rich adapter library (Redis, OTLP, Prometheus, etc.)
- Multiple control plane transports
- Production recipes and patterns

---

## Let's Go. HOO RAH.
