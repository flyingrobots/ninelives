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

## Appendix

### [P0.01] timeout.rs -> TimeoutLayer + TimeoutService
- Steps:
  - Plan timeout.rs -> TimeoutLayer + TimeoutService
  - Implement & tests
  - Document
- Estimate: 2-3h
- Scope: In scope: timeout.rs -> TimeoutLayer + TimeoutService. Out of scope: unrelated transports/services.
- Complexity: ~80 LoC

### [P0.02] retry.rs -> RetryLayer + RetryService
- Steps:
  - Plan retry.rs -> RetryLayer + RetryService
  - Implement & tests
  - Document
- Estimate: 2-3h
- Scope: In scope: retry.rs -> RetryLayer + RetryService. Out of scope: unrelated transports/services.
- Complexity: ~80 LoC

### [P0.03] bulkhead.rs -> BulkheadLayer + BulkheadService
- Steps:
  - Plan bulkhead.rs -> BulkheadLayer + BulkheadService
  - Implement & tests
  - Document
- Estimate: 2-3h
- Scope: In scope: bulkhead.rs -> BulkheadLayer + BulkheadService. Out of scope: unrelated transports/services.
- Complexity: ~80 LoC

### [P0.04] circuit_breaker.rs -> CircuitBreakerLayer + CircuitBreakerService
- Steps:
  - Plan circuit_breaker.rs -> CircuitBreakerLayer + CircuitBreakerService
  - Implement & tests
  - Document
- Estimate: 2-3h
- Scope: In scope: circuit_breaker.rs -> CircuitBreakerLayer + CircuitBreakerService. Out of scope: unrelated transports/services.
- Complexity: ~80 LoC

### [P0.05] Backoff/Jitter integration with retry layer
- Steps:
  - Plan Backoff/Jitter integration with retry layer
  - Implement & tests
  - Document
- Estimate: 2-3h
- Scope: In scope: Backoff/Jitter integration with retry layer. Out of scope: unrelated transports/services.
- Complexity: ~80 LoC

### [P0.06] Policy wrapper
- Steps:
  - Plan Policy wrapper
  - Implement & tests
  - Document
- Estimate: 2-3h
- Scope: In scope: Policy wrapper. Out of scope: unrelated transports/services.
- Complexity: ~80 LoC

### [P0.07] `+` operator (CombinedLayer for sequential composition)
- Steps:
  - Plan `+` operator (CombinedLayer for sequential composition)
  - Implement & tests
  - Document
- Estimate: 2-3h
- Scope: In scope: `+` operator (CombinedLayer for sequential composition). Out of scope: unrelated transports/services.
- Complexity: ~80 LoC

### [P0.08] `|` operator (FallbackLayer for failover)
- Steps:
  - Plan `|` operator (FallbackLayer for failover)
  - Implement & tests
  - Document
- Estimate: 2-3h
- Scope: In scope: `|` operator (FallbackLayer for failover). Out of scope: unrelated transports/services.
- Complexity: ~80 LoC

### [P0.09] Doc comments and examples for algebra API
- Steps:
  - Plan Doc comments and examples for algebra API
  - Implement & tests
  - Document
- Estimate: 1-2h
- Scope: In scope: Doc comments and examples for algebra API. Out of scope: unrelated transports/services.
- Complexity: ~40 lines

### [P0.10] Operator precedence documentation
- Steps:
  - Plan Operator precedence documentation
  - Implement & tests
  - Document
- Estimate: 1-2h
- Scope: In scope: Operator precedence documentation. Out of scope: unrelated transports/services.
- Complexity: ~40 lines

### [P0.11] Delete src/stack.rs
- Steps:
  - Plan Delete src/stack.rs
  - Implement & tests
  - Document
- Estimate: 2-3h
- Scope: In scope: Delete src/stack.rs. Out of scope: unrelated transports/services.
- Complexity: ~80 LoC

### [P0.12] Remove ResilienceStack exports from lib/prelude
- Steps:
  - Plan Remove ResilienceStack exports from lib/prelude
  - Implement & tests
  - Document
- Estimate: 2-3h
- Scope: In scope: Remove ResilienceStack exports from lib/prelude. Out of scope: unrelated transports/services.
- Complexity: ~80 LoC

### [P0.13] Delete examples/full_stack.rs
- Steps:
  - Plan Delete examples/full_stack.rs
  - Implement & tests
  - Document
- Estimate: 2-3h
- Scope: In scope: Delete examples/full_stack.rs. Out of scope: unrelated transports/services.
- Complexity: ~80 LoC

### [P0.14] Remove legacy references from tests
- Steps:
  - Plan Remove legacy references from tests
  - Implement & tests
  - Document
- Estimate: 2-3h
- Scope: In scope: Remove legacy references from tests. Out of scope: unrelated transports/services.
- Complexity: ~80 LoC

### [P0.15] Update lib.rs with new quick start (Policy + tower ServiceBuilder)
- Steps:
  - Plan Update lib.rs with new quick start (Policy + tower ServiceBuilder)
  - Implement & tests
  - Document
- Estimate: 2-4h
- Scope: In scope: Update lib.rs with new quick start (Policy + tower ServiceBuilder). Out of scope: unrelated transports/services.
- Complexity: ~100 LoC

### [P0.16] Update prelude.rs with algebra re-exports
- Steps:
  - Plan Update prelude.rs with algebra re-exports
  - Implement & tests
  - Document
- Estimate: 2-4h
- Scope: In scope: Update prelude.rs with algebra re-exports. Out of scope: unrelated transports/services.
- Complexity: ~100 LoC

### [P0.17] Create README.md with:
- Steps:
  - Plan Create README.md with:
  - Implement & tests
  - Document
- Estimate: 2-4h
- Scope: In scope: Create README.md with:. Out of scope: unrelated transports/services.
- Complexity: ~100 LoC

### [P0.18] Update examples/:
- Steps:
  - Plan Update examples/:
  - Implement & tests
  - Document
- Estimate: 2-4h
- Scope: In scope: Update examples/:. Out of scope: unrelated transports/services.
- Complexity: ~100 LoC

### [P0.19] Add doc tests for algebra operators
- Steps:
  - Plan Add doc tests for algebra operators
  - Implement & tests
  - Document
- Estimate: 1-2h
- Scope: In scope: Add doc tests for algebra operators. Out of scope: unrelated transports/services.
- Complexity: ~40 lines

### [P0.20] Adapt integration tests to Layer/Service architecture
- Steps:
  - Plan Adapt integration tests to Layer/Service architecture
  - Implement & tests
  - Document
- Estimate: 2-3h
- Scope: In scope: Adapt integration tests to Layer/Service architecture. Out of scope: unrelated transports/services.
- Complexity: ~80 LoC

### [P0.21] Add test coverage for:
- Steps:
  - Plan Add test coverage for:
  - Implement & tests
  - Document
- Estimate: 2-4h
- Scope: In scope: Add test coverage for:. Out of scope: unrelated transports/services.
- Complexity: ~100 LoC

### [P0.22] Ensure clippy passes
- Steps:
  - Plan Ensure clippy passes
  - Implement & tests
  - Document
- Estimate: 2-3h
- Scope: In scope: Ensure clippy passes. Out of scope: unrelated transports/services.
- Complexity: ~80 LoC

### [P0.23] Ensure all doc tests pass
- Steps:
  - Plan Ensure all doc tests pass
  - Implement & tests
  - Document
- Estimate: 1-2h
- Scope: In scope: Ensure all doc tests pass. Out of scope: unrelated transports/services.
- Complexity: ~40 lines

### [P0.24] Add CI workflow if missing
- Steps:
  - Plan Add CI workflow if missing
  - Implement & tests
  - Document
- Estimate: 2-4h
- Scope: In scope: Add CI workflow if missing. Out of scope: unrelated transports/services.
- Complexity: ~100 LoC

### [P1.01] Define `PolicyEvent` enum:
- Steps:
  - Plan Define `PolicyEvent` enum:
  - Implement & tests
  - Document
- Estimate: 4-6h
- Scope: In scope: Define `PolicyEvent` enum:. Out of scope: unrelated transports/services.
- Complexity: ~120 LoC

### [P1.02] Add event emission to all policy layers:
- Steps:
  - Plan Add event emission to all policy layers:
  - Implement & tests
  - Document
- Estimate: 2-4h
- Scope: In scope: Add event emission to all policy layers:. Out of scope: unrelated transports/services.
- Complexity: ~100 LoC

### [P1.03] Define `TelemetrySink` as a `tower::Service<PolicyEvent, Response=(), Error=E>`
- Steps:
  - Plan Define `TelemetrySink` as a `tower::Service<PolicyEvent, Response=(), Error=E>`
  - Implement & tests
  - Document
- Estimate: 4-6h
- Scope: In scope: Define `TelemetrySink` as a `tower::Service<PolicyEvent, Response=(), Error=E>`. Out of scope: unrelated transports/services.
- Complexity: ~120 LoC

### [P1.04] Implement basic sinks:
- Steps:
  - Plan Implement basic sinks:
  - Implement & tests
  - Document
- Estimate: 2-4h
- Scope: In scope: Implement basic sinks:. Out of scope: unrelated transports/services.
- Complexity: ~100 LoC

### [P1.05] Wire policies to accept telemetry sink via `.with_sink()` method
- Steps:
  - Plan Wire policies to accept telemetry sink via `.with_sink()` method
  - Implement & tests
  - Document
- Estimate: 2-3h
- Scope: In scope: Wire policies to accept telemetry sink via `.with_sink()` method. Out of scope: unrelated transports/services.
- Complexity: ~80 LoC

### [P1.06] Implement `MulticastSink` for sending to multiple sinks (multicast to both)
- Steps:
  - Plan Implement `MulticastSink` for sending to multiple sinks (multicast to both)
  - Implement & tests
  - Document
- Estimate: 2-4h
- Scope: In scope: Implement `MulticastSink` for sending to multiple sinks (multicast to both). Out of scope: unrelated transports/services.
- Complexity: ~100 LoC

### [P1.07] Implement `FallbackSink` for fallback on failure
- Steps:
  - Plan Implement `FallbackSink` for fallback on failure
  - Implement & tests
  - Document
- Estimate: 2-4h
- Scope: In scope: Implement `FallbackSink` for fallback on failure. Out of scope: unrelated transports/services.
- Complexity: ~100 LoC

### [P1.08] Add `ComposedSinkError` type for composition errors
- Steps:
  - Plan Add `ComposedSinkError` type for composition errors
  - Implement & tests
  - Document
- Estimate: 2-4h
- Scope: In scope: Add `ComposedSinkError` type for composition errors. Out of scope: unrelated transports/services.
- Complexity: ~100 LoC

### [P1.09] Document sink composition patterns
- Steps:
  - Plan Document sink composition patterns
  - Implement & tests
  - Document
- Estimate: 1-2h
- Scope: In scope: Document sink composition patterns. Out of scope: unrelated transports/services.
- Complexity: ~40 lines

### [P1.10] Thread sink through policy constructors/builders via `.with_sink()` method
- Steps:
  - Plan Thread sink through policy constructors/builders via `.with_sink()` method
  - Implement & tests
  - Document
- Estimate: 2-3h
- Scope: In scope: Thread sink through policy constructors/builders via `.with_sink()` method. Out of scope: unrelated transports/services.
- Complexity: ~80 LoC

### [P1.11] Add examples showing telemetry integration:
- Steps:
  - Plan Add examples showing telemetry integration:
  - Implement & tests
  - Document
- Estimate: 2-4h
- Scope: In scope: Add examples showing telemetry integration:. Out of scope: unrelated transports/services.
- Complexity: ~100 LoC

### [P1.12] Benchmark overhead of event emission (deferred to Phase 10)
- Steps:
  - Plan Benchmark overhead of event emission (deferred to Phase 10)
  - Implement & tests
  - Document
- Estimate: 3-5h
- Scope: In scope: Benchmark overhead of event emission (deferred to Phase 10). Out of scope: unrelated transports/services.
- Complexity: ~80 LoC + scripts

### [P2.01] Design `Adaptive<T>` wrapper:
- Steps:
  - Plan Design `Adaptive<T>` wrapper:
  - Implement & tests
  - Document
- Estimate: 4-6h
- Scope: In scope: Design `Adaptive<T>` wrapper:. Out of scope: unrelated transports/services.
- Complexity: ~120 LoC

### [P2.02] Integrate Adaptive into policy configs:
- Steps:
  - Plan Integrate Adaptive into policy configs:
  - Implement & tests
  - Document
- Estimate: 2-4h
- Scope: In scope: Integrate Adaptive into policy configs:. Out of scope: unrelated transports/services.
- Complexity: ~100 LoC

### [P2.03] Define `CommandContext` struct:
- Steps:
  - Plan Define `CommandContext` struct:
  - Implement & tests
  - Document
- Estimate: 4-6h
- Scope: In scope: Define `CommandContext` struct:. Out of scope: unrelated transports/services.
- Complexity: ~120 LoC

### [P2.04] Define `CommandHandler` trait as `tower::Service<CommandContext>`
- Steps:
  - Plan Define `CommandHandler` trait as `tower::Service<CommandContext>`
  - Implement & tests
  - Document
- Estimate: 4-6h
- Scope: In scope: Define `CommandHandler` trait as `tower::Service<CommandContext>`. Out of scope: unrelated transports/services.
- Complexity: ~120 LoC

### [P2.05] Implement `ControlPlaneRouter`:
- Steps:
  - Plan Implement `ControlPlaneRouter`:
  - Implement & tests
  - Document
- Estimate: 2-4h
- Scope: In scope: Implement `ControlPlaneRouter`:. Out of scope: unrelated transports/services.
- Complexity: ~100 LoC

### [P2.06] `SetParameterHandler` (update Adaptive values)
- Steps:
  - Plan `SetParameterHandler` (update Adaptive values)
  - Implement & tests
  - Document
- Estimate: 2-4h
- Scope: In scope: `SetParameterHandler` (update Adaptive values). Out of scope: unrelated transports/services.
- Complexity: ~100 LoC

### [P2.07] `GetParameterHandler` (read current config)
- Steps:
  - Plan `GetParameterHandler` (read current config)
  - Implement & tests
  - Document
- Estimate: 2-3h
- Scope: In scope: `GetParameterHandler` (read current config). Out of scope: unrelated transports/services.
- Complexity: ~80 LoC

### [P2.08] `GetStateHandler` (query policy state)
- Steps:
  - Plan `GetStateHandler` (query policy state)
  - Implement & tests
  - Document
- Estimate: 2-3h
- Scope: In scope: `GetStateHandler` (query policy state). Out of scope: unrelated transports/services.
- Complexity: ~80 LoC

### [P2.09] `ResetCircuitBreakerHandler`
- Steps:
  - Plan `ResetCircuitBreakerHandler`
  - Implement & tests
  - Document
- Estimate: 2-3h
- Scope: In scope: `ResetCircuitBreakerHandler`. Out of scope: unrelated transports/services.
- Complexity: ~80 LoC

### [P2.10] `ListPoliciesHandler`
- Steps:
  - Plan `ListPoliciesHandler`
  - Implement & tests
  - Document
- Estimate: 2-3h
- Scope: In scope: `ListPoliciesHandler`. Out of scope: unrelated transports/services.
- Complexity: ~80 LoC

### [P2.11] Start with `ninelives-control` crate
- Steps:
  - Plan Start with `ninelives-control` crate
  - Implement & tests
  - Document
- Estimate: 2-3h
- Scope: In scope: Start with `ninelives-control` crate. Out of scope: unrelated transports/services.
- Complexity: ~80 LoC

### [P2.12] Implement local/in-process transport (channels)
- Steps:
  - Plan Implement local/in-process transport (channels)
  - Implement & tests
  - Document
- Estimate: 2-4h
- Scope: In scope: Implement local/in-process transport (channels). Out of scope: unrelated transports/services.
- Complexity: ~100 LoC

### [P2.13] Design transport abstraction for future HTTP/gRPC/etc.
- Steps:
  - Plan Design transport abstraction for future HTTP/gRPC/etc.
  - Implement & tests
  - Document
- Estimate: 4-6h
- Scope: In scope: Design transport abstraction for future HTTP/gRPC/etc.. Out of scope: unrelated transports/services.
- Complexity: ~120 LoC

### [P2.14] Define `AuthorizationLayer` (checks Identity in CommandContext)
- Steps:
  - Plan Define `AuthorizationLayer` (checks Identity in CommandContext)
  - Implement & tests
  - Document
- Estimate: 4-6h
- Scope: In scope: Define `AuthorizationLayer` (checks Identity in CommandContext). Out of scope: unrelated transports/services.
- Complexity: ~120 LoC

### [P2.15] Define `AuditLayer` (logs all commands)
- Steps:
  - Plan Define `AuditLayer` (logs all commands)
  - Implement & tests
  - Document
- Estimate: 4-6h
- Scope: In scope: Define `AuditLayer` (logs all commands). Out of scope: unrelated transports/services.
- Complexity: ~120 LoC

### [P2.16] Wrap ControlPlaneRouter in Policy(AuthZ) + Policy(Audit)
- Steps:
  - Plan Wrap ControlPlaneRouter in Policy(AuthZ) + Policy(Audit)
  - Implement & tests
  - Document
- Estimate: 2-3h
- Scope: In scope: Wrap ControlPlaneRouter in Policy(AuthZ) + Policy(Audit). Out of scope: unrelated transports/services.
- Complexity: ~80 LoC

### [P3.01] Design `SystemState` struct:
- Steps:
  - Plan Design `SystemState` struct:
  - Implement & tests
  - Document
- Estimate: 4-6h
- Scope: In scope: Design `SystemState` struct:. Out of scope: unrelated transports/services.
- Complexity: ~120 LoC

### [P3.02] Implement efficient storage (ring buffers, sketches)
- Steps:
  - Plan Implement efficient storage (ring buffers, sketches)
  - Implement & tests
  - Document
- Estimate: 2-4h
- Scope: In scope: Implement efficient storage (ring buffers, sketches). Out of scope: unrelated transports/services.
- Complexity: ~100 LoC

### [P3.03] Create `ninelives-observer` crate
- Steps:
  - Plan Create `ninelives-observer` crate
  - Implement & tests
  - Document
- Estimate: 2-4h
- Scope: In scope: Create `ninelives-observer` crate. Out of scope: unrelated transports/services.
- Complexity: ~100 LoC

### [P3.04] Implement `Observer` as a background task
- Steps:
  - Plan Implement `Observer` as a background task
  - Implement & tests
  - Document
- Estimate: 2-4h
- Scope: In scope: Implement `Observer` as a background task. Out of scope: unrelated transports/services.
- Complexity: ~100 LoC

### [P3.05] Subscribe to StreamingSink
- Steps:
  - Plan Subscribe to StreamingSink
  - Implement & tests
  - Document
- Estimate: 2-3h
- Scope: In scope: Subscribe to StreamingSink. Out of scope: unrelated transports/services.
- Complexity: ~80 LoC

### [P3.06] Ingest PolicyEvents and update SystemState
- Steps:
  - Plan Ingest PolicyEvents and update SystemState
  - Implement & tests
  - Document
- Estimate: 2-4h
- Scope: In scope: Ingest PolicyEvents and update SystemState. Out of scope: unrelated transports/services.
- Complexity: ~100 LoC

### [P3.07] Expose query interface:
- Steps:
  - Plan Expose query interface:
  - Implement & tests
  - Document
- Estimate: 2-3h
- Scope: In scope: Expose query interface:. Out of scope: unrelated transports/services.
- Complexity: ~80 LoC

### [P3.08] Wire Observer to telemetry message bus
- Steps:
  - Plan Wire Observer to telemetry message bus
  - Implement & tests
  - Document
- Estimate: 2-3h
- Scope: In scope: Wire Observer to telemetry message bus. Out of scope: unrelated transports/services.
- Complexity: ~80 LoC

### [P3.09] Add control plane commands to query Observer state
- Steps:
  - Plan Add control plane commands to query Observer state
  - Implement & tests
  - Document
- Estimate: 2-4h
- Scope: In scope: Add control plane commands to query Observer state. Out of scope: unrelated transports/services.
- Complexity: ~100 LoC

### [P3.10] Add examples showing Observer usage
- Steps:
  - Plan Add examples showing Observer usage
  - Implement & tests
  - Document
- Estimate: 2-4h
- Scope: In scope: Add examples showing Observer usage. Out of scope: unrelated transports/services.
- Complexity: ~100 LoC

### [P4.01] Design `ForkJoinLayer` and `ForkJoinService`
- Steps:
  - Plan Design `ForkJoinLayer` and `ForkJoinService`
  - Implement & tests
  - Document
- Estimate: 4-6h
- Scope: In scope: Design `ForkJoinLayer` and `ForkJoinService`. Out of scope: unrelated transports/services.
- Complexity: ~120 LoC

### [P4.02] Spawn both services concurrently (futures::select for racing)
- Steps:
  - Plan Spawn both services concurrently (futures::select for racing)
  - Implement & tests
  - Document
- Estimate: 2-3h
- Scope: In scope: Spawn both services concurrently (futures::select for racing). Out of scope: unrelated transports/services.
- Complexity: ~80 LoC

### [P4.03] Return first `Ok` result
- Steps:
  - Plan Return first `Ok` result
  - Implement & tests
  - Document
- Estimate: 2-3h
- Scope: In scope: Return first `Ok` result. Out of scope: unrelated transports/services.
- Complexity: ~80 LoC

### [P4.04] Cancel remaining futures on first success
- Steps:
  - Plan Cancel remaining futures on first success
  - Implement & tests
  - Document
- Estimate: 2-3h
- Scope: In scope: Cancel remaining futures on first success. Out of scope: unrelated transports/services.
- Complexity: ~80 LoC

### [P4.05] Handle case where both fail (return error)
- Steps:
  - Plan Handle case where both fail (return error)
  - Implement & tests
  - Document
- Estimate: 2-3h
- Scope: In scope: Handle case where both fail (return error). Out of scope: unrelated transports/services.
- Complexity: ~80 LoC

### [P4.06] Implement `BitAnd` trait for `Policy<L>`
- Steps:
  - Plan Implement `BitAnd` trait for `Policy<L>`
  - Implement & tests
  - Document
- Estimate: 2-4h
- Scope: In scope: Implement `BitAnd` trait for `Policy<L>`. Out of scope: unrelated transports/services.
- Complexity: ~100 LoC

### [P4.07] Returns `Policy<ForkJoinLayer<A, B>>`
- Steps:
  - Plan Returns `Policy<ForkJoinLayer<A, B>>`
  - Implement & tests
  - Document
- Estimate: 2-3h
- Scope: In scope: Returns `Policy<ForkJoinLayer<A, B>>`. Out of scope: unrelated transports/services.
- Complexity: ~80 LoC

### [P4.08] Test race conditions (doc tests cover both sides)
- Steps:
  - Plan Test race conditions (doc tests cover both sides)
  - Implement & tests
  - Document
- Estimate: 1-2h
- Scope: In scope: Test race conditions (doc tests cover both sides). Out of scope: unrelated transports/services.
- Complexity: ~40 lines

### [P4.09] Test both-fail scenarios (implemented in service logic)
- Steps:
  - Plan Test both-fail scenarios (implemented in service logic)
  - Implement & tests
  - Document
- Estimate: 2-4h
- Scope: In scope: Test both-fail scenarios (implemented in service logic). Out of scope: unrelated transports/services.
- Complexity: ~100 LoC

### [P4.10] Test cancellation behavior (futures::select handles drop)
- Steps:
  - Plan Test cancellation behavior (futures::select handles drop)
  - Implement & tests
  - Document
- Estimate: 2-3h
- Scope: In scope: Test cancellation behavior (futures::select handles drop). Out of scope: unrelated transports/services.
- Complexity: ~80 LoC

### [P4.11] Benchmark overhead vs sequential
- Steps:
  - Plan Benchmark overhead vs sequential
  - Implement & tests
  - Document
- Estimate: 3-5h
- Scope: In scope: Benchmark overhead vs sequential. Out of scope: unrelated transports/services.
- Complexity: ~80 LoC + scripts

### [P4.12] Add examples: IPv4/IPv6, cache strategies
- Steps:
  - Plan Add examples: IPv4/IPv6, cache strategies
  - Implement & tests
  - Document
- Estimate: 2-4h
- Scope: In scope: Add examples: IPv4/IPv6, cache strategies. Out of scope: unrelated transports/services.
- Complexity: ~100 LoC

### [P4.13] Document operator precedence: `&` > `+` > `|`
- Steps:
  - Plan Document operator precedence: `&` > `+` > `|`
  - Implement & tests
  - Document
- Estimate: 1-2h
- Scope: In scope: Document operator precedence: `&` > `+` > `|`. Out of scope: unrelated transports/services.
- Complexity: ~40 lines

### [P4.14] Add to algebra guide (README, lib.rs, examples)
- Steps:
  - Plan Add to algebra guide (README, lib.rs, examples)
  - Implement & tests
  - Document
- Estimate: 2-4h
- Scope: In scope: Add to algebra guide (README, lib.rs, examples). Out of scope: unrelated transports/services.
- Complexity: ~100 LoC

### [P5.01] Create `ninelives-sentinel` crate
- Steps:
  - Plan Create `ninelives-sentinel` crate
  - Implement & tests
  - Document
- Estimate: 2-4h
- Scope: In scope: Create `ninelives-sentinel` crate. Out of scope: unrelated transports/services.
- Complexity: ~100 LoC

### [P5.02] Integrate Rhai scripting engine
- Steps:
  - Plan Integrate Rhai scripting engine
  - Implement & tests
  - Document
- Estimate: 2-4h
- Scope: In scope: Integrate Rhai scripting engine. Out of scope: unrelated transports/services.
- Complexity: ~100 LoC

### [P5.03] Define script API:
- Steps:
  - Plan Define script API:
  - Implement & tests
  - Document
- Estimate: 4-6h
- Scope: In scope: Define script API:. Out of scope: unrelated transports/services.
- Complexity: ~120 LoC

### [P5.04] Implement meta-policy evaluation loop:
- Steps:
  - Plan Implement meta-policy evaluation loop:
  - Implement & tests
  - Document
- Estimate: 2-4h
- Scope: In scope: Implement meta-policy evaluation loop:. Out of scope: unrelated transports/services.
- Complexity: ~100 LoC

### [P5.05] `ReloadMetaPolicyHandler` command
- Steps:
  - Plan `ReloadMetaPolicyHandler` command
  - Implement & tests
  - Document
- Estimate: 3-5h
- Scope: In scope: `ReloadMetaPolicyHandler` command. Out of scope: unrelated transports/services.
- Complexity: ~80 LoC + scripts

### [P5.06] Watch script file for changes (optional)
- Steps:
  - Plan Watch script file for changes (optional)
  - Implement & tests
  - Document
- Estimate: 2-3h
- Scope: In scope: Watch script file for changes (optional). Out of scope: unrelated transports/services.
- Complexity: ~80 LoC

### [P5.07] Validate script before activating
- Steps:
  - Plan Validate script before activating
  - Implement & tests
  - Document
- Estimate: 2-3h
- Scope: In scope: Validate script before activating. Out of scope: unrelated transports/services.
- Complexity: ~80 LoC

### [P5.08] Auto-adjust retry backoff based on error rate
- Steps:
  - Plan Auto-adjust retry backoff based on error rate
  - Implement & tests
  - Document
- Estimate: 2-3h
- Scope: In scope: Auto-adjust retry backoff based on error rate. Out of scope: unrelated transports/services.
- Complexity: ~80 LoC

### [P5.09] Open circuit breaker on sustained failures
- Steps:
  - Plan Open circuit breaker on sustained failures
  - Implement & tests
  - Document
- Estimate: 2-3h
- Scope: In scope: Open circuit breaker on sustained failures. Out of scope: unrelated transports/services.
- Complexity: ~80 LoC

### [P5.10] Increase bulkhead capacity under load
- Steps:
  - Plan Increase bulkhead capacity under load
  - Implement & tests
  - Document
- Estimate: 3-5h
- Scope: In scope: Increase bulkhead capacity under load. Out of scope: unrelated transports/services.
- Complexity: ~80 LoC + scripts

### [P5.11] Alert on anomalies
- Steps:
  - Plan Alert on anomalies
  - Implement & tests
  - Document
- Estimate: 2-3h
- Scope: In scope: Alert on anomalies. Out of scope: unrelated transports/services.
- Complexity: ~80 LoC

### [P5.12] Implement `Sentinel` as top-level coordinator:
- Steps:
  - Plan Implement `Sentinel` as top-level coordinator:
  - Implement & tests
  - Document
- Estimate: 2-4h
- Scope: In scope: Implement `Sentinel` as top-level coordinator:. Out of scope: unrelated transports/services.
- Complexity: ~100 LoC

### [P5.13] Add graceful shutdown
- Steps:
  - Plan Add graceful shutdown
  - Implement & tests
  - Document
- Estimate: 2-4h
- Scope: In scope: Add graceful shutdown. Out of scope: unrelated transports/services.
- Complexity: ~100 LoC

### [P6.01] Extend Adaptive<T> to support shadow values:
- Steps:
  - Plan Extend Adaptive<T> to support shadow values:
  - Implement & tests
  - Document
- Estimate: 2-3h
- Scope: In scope: Extend Adaptive<T> to support shadow values:. Out of scope: unrelated transports/services.
- Complexity: ~80 LoC

### [P6.02] Add shadow mode to policy layers:
- Steps:
  - Plan Add shadow mode to policy layers:
  - Implement & tests
  - Document
- Estimate: 2-4h
- Scope: In scope: Add shadow mode to policy layers:. Out of scope: unrelated transports/services.
- Complexity: ~100 LoC

### [P6.03] Define `ShadowEvent` type (includes primary + shadow results)
- Steps:
  - Plan Define `ShadowEvent` type (includes primary + shadow results)
  - Implement & tests
  - Document
- Estimate: 4-6h
- Scope: In scope: Define `ShadowEvent` type (includes primary + shadow results). Out of scope: unrelated transports/services.
- Complexity: ~120 LoC

### [P6.04] Add shadow event emission to policies
- Steps:
  - Plan Add shadow event emission to policies
  - Implement & tests
  - Document
- Estimate: 2-4h
- Scope: In scope: Add shadow event emission to policies. Out of scope: unrelated transports/services.
- Complexity: ~100 LoC

### [P6.05] Observer ingests shadow events separately
- Steps:
  - Plan Observer ingests shadow events separately
  - Implement & tests
  - Document
- Estimate: 2-3h
- Scope: In scope: Observer ingests shadow events separately. Out of scope: unrelated transports/services.
- Complexity: ~80 LoC

### [P6.06] Sentinel observes shadow stability over time window
- Steps:
  - Plan Sentinel observes shadow stability over time window
  - Implement & tests
  - Document
- Estimate: 2-3h
- Scope: In scope: Sentinel observes shadow stability over time window. Out of scope: unrelated transports/services.
- Complexity: ~80 LoC

### [P6.07] Issues `PromoteShadowHandler` command if stable
- Steps:
  - Plan Issues `PromoteShadowHandler` command if stable
  - Implement & tests
  - Document
- Estimate: 2-3h
- Scope: In scope: Issues `PromoteShadowHandler` command if stable. Out of scope: unrelated transports/services.
- Complexity: ~80 LoC

### [P6.08] Policies atomically swap shadow -> primary
- Steps:
  - Plan Policies atomically swap shadow -> primary
  - Implement & tests
  - Document
- Estimate: 2-3h
- Scope: In scope: Policies atomically swap shadow -> primary. Out of scope: unrelated transports/services.
- Complexity: ~80 LoC

### [P6.09] Ensure shadow evaluation doesn't affect primary path latency
- Steps:
  - Plan Ensure shadow evaluation doesn't affect primary path latency
  - Implement & tests
  - Document
- Estimate: 2-3h
- Scope: In scope: Ensure shadow evaluation doesn't affect primary path latency. Out of scope: unrelated transports/services.
- Complexity: ~80 LoC

### [P6.10] Add circuit breaker to kill shadow eval if too expensive
- Steps:
  - Plan Add circuit breaker to kill shadow eval if too expensive
  - Implement & tests
  - Document
- Estimate: 2-4h
- Scope: In scope: Add circuit breaker to kill shadow eval if too expensive. Out of scope: unrelated transports/services.
- Complexity: ~100 LoC

### [P6.11] Document safety guarantees
- Steps:
  - Plan Document safety guarantees
  - Implement & tests
  - Document
- Estimate: 1-2h
- Scope: In scope: Document safety guarantees. Out of scope: unrelated transports/services.
- Complexity: ~40 lines

### [P7.01] Create workspace Cargo.toml
- Steps:
  - Plan Create workspace Cargo.toml
  - Implement & tests
  - Document
- Estimate: 2-4h
- Scope: In scope: Create workspace Cargo.toml. Out of scope: unrelated transports/services.
- Complexity: ~100 LoC

### [P7.02] Split crates:
- Steps:
  - Plan Split crates:
  - Implement & tests
  - Document
- Estimate: 2-3h
- Scope: In scope: Split crates:. Out of scope: unrelated transports/services.
- Complexity: ~80 LoC

### [P7.03] Update dependencies and re-exports
- Steps:
  - Plan Update dependencies and re-exports
  - Implement & tests
  - Document
- Estimate: 2-4h
- Scope: In scope: Update dependencies and re-exports. Out of scope: unrelated transports/services.
- Complexity: ~100 LoC

### [P7.04] Ensure backward compatibility
- Steps:
  - Plan Ensure backward compatibility
  - Implement & tests
  - Document
- Estimate: 2-3h
- Scope: In scope: Ensure backward compatibility. Out of scope: unrelated transports/services.
- Complexity: ~80 LoC

### [P7.05] Create adapter template/guide
- Steps:
  - Plan Create adapter template/guide
  - Implement & tests
  - Document
- Estimate: 2-4h
- Scope: In scope: Create adapter template/guide. Out of scope: unrelated transports/services.
- Complexity: ~100 LoC

### [P7.06] Implement priority adapters:
- Steps:
  - Plan Implement priority adapters:
  - Implement & tests
  - Document
- Estimate: 2-4h
- Scope: In scope: Implement priority adapters:. Out of scope: unrelated transports/services.
- Complexity: ~100 LoC

### [P7.07] Document adapter development
- Steps:
  - Plan Document adapter development
  - Implement & tests
  - Document
- Estimate: 1-2h
- Scope: In scope: Document adapter development. Out of scope: unrelated transports/services.
- Complexity: ~40 lines

### [P8.01] Design transport-agnostic command serialization
- Steps:
  - Plan Design transport-agnostic command serialization
  - Implement & tests
  - Document
- Estimate: 4-6h
- Scope: In scope: Design transport-agnostic command serialization. Out of scope: unrelated transports/services.
- Complexity: ~120 LoC

### [P8.02] Support JSON and/or MessagePack
- Steps:
  - Plan Support JSON and/or MessagePack
  - Implement & tests
  - Document
- Estimate: 2-3h
- Scope: In scope: Support JSON and/or MessagePack. Out of scope: unrelated transports/services.
- Complexity: ~80 LoC

### [P8.03] Create `ninelives-rest` crate
- Steps:
  - Plan Create `ninelives-rest` crate
  - Implement & tests
  - Document
- Estimate: 2-4h
- Scope: In scope: Create `ninelives-rest` crate. Out of scope: unrelated transports/services.
- Complexity: ~100 LoC

### [P8.04] Expose ControlPlaneRouter over HTTP endpoints
- Steps:
  - Plan Expose ControlPlaneRouter over HTTP endpoints
  - Implement & tests
  - Document
- Estimate: 2-3h
- Scope: In scope: Expose ControlPlaneRouter over HTTP endpoints. Out of scope: unrelated transports/services.
- Complexity: ~80 LoC

### [P8.05] Add authentication middleware
- Steps:
  - Plan Add authentication middleware
  - Implement & tests
  - Document
- Estimate: 2-4h
- Scope: In scope: Add authentication middleware. Out of scope: unrelated transports/services.
- Complexity: ~100 LoC

### [P8.06] `ninelives-graphql` (GraphQL API)
- Steps:
  - Plan `ninelives-graphql` (GraphQL API)
  - Implement & tests
  - Document
- Estimate: 2-3h
- Scope: In scope: `ninelives-graphql` (GraphQL API). Out of scope: unrelated transports/services.
- Complexity: ~80 LoC

### [P8.07] `ninelives-mcp` (Model Context Protocol)
- Steps:
  - Plan `ninelives-mcp` (Model Context Protocol)
  - Implement & tests
  - Document
- Estimate: 2-3h
- Scope: In scope: `ninelives-mcp` (Model Context Protocol). Out of scope: unrelated transports/services.
- Complexity: ~80 LoC

### [P8.08] `ninelives-grpc` (gRPC service)
- Steps:
  - Plan `ninelives-grpc` (gRPC service)
  - Implement & tests
  - Document
- Estimate: 2-3h
- Scope: In scope: `ninelives-grpc` (gRPC service). Out of scope: unrelated transports/services.
- Complexity: ~80 LoC

### [P9.01] Autonomous Canary Releases
- Steps:
  - Plan Autonomous Canary Releases
  - Implement & tests
  - Document
- Estimate: 2-3h
- Scope: In scope: Autonomous Canary Releases. Out of scope: unrelated transports/services.
- Complexity: ~80 LoC

### [P9.02] Progressive Ratchet-Up
- Steps:
  - Plan Progressive Ratchet-Up
  - Implement & tests
  - Document
- Estimate: 2-3h
- Scope: In scope: Progressive Ratchet-Up. Out of scope: unrelated transports/services.
- Complexity: ~80 LoC

### [P9.03] Safety Valves (auto-scaling policies)
- Steps:
  - Plan Safety Valves (auto-scaling policies)
  - Implement & tests
  - Document
- Estimate: 2-3h
- Scope: In scope: Safety Valves (auto-scaling policies). Out of scope: unrelated transports/services.
- Complexity: ~80 LoC

### [P9.04] Blue/Green Deployments
- Steps:
  - Plan Blue/Green Deployments
  - Implement & tests
  - Document
- Estimate: 2-3h
- Scope: In scope: Blue/Green Deployments. Out of scope: unrelated transports/services.
- Complexity: ~80 LoC

### [P9.05] Multi-Region Failover
- Steps:
  - Plan Multi-Region Failover
  - Implement & tests
  - Document
- Estimate: 2-3h
- Scope: In scope: Multi-Region Failover. Out of scope: unrelated transports/services.
- Complexity: ~80 LoC

### [P9.06] Build example apps in `examples/recipes/`
- Steps:
  - Plan Build example apps in `examples/recipes/`
  - Implement & tests
  - Document
- Estimate: 2-3h
- Scope: In scope: Build example apps in `examples/recipes/`. Out of scope: unrelated transports/services.
- Complexity: ~80 LoC

### [P9.07] Include Sentinel scripts for each pattern
- Steps:
  - Plan Include Sentinel scripts for each pattern
  - Implement & tests
  - Document
- Estimate: 2-3h
- Scope: In scope: Include Sentinel scripts for each pattern. Out of scope: unrelated transports/services.
- Complexity: ~80 LoC

### [P9.08] Add integration tests
- Steps:
  - Plan Add integration tests
  - Implement & tests
  - Document
- Estimate: 2-4h
- Scope: In scope: Add integration tests. Out of scope: unrelated transports/services.
- Complexity: ~100 LoC

### [P10.01] Criterion benchmarks for each layer
- Steps:
  - Plan Criterion benchmarks for each layer
  - Implement & tests
  - Document
- Estimate: 3-5h
- Scope: In scope: Criterion benchmarks for each layer. Out of scope: unrelated transports/services.
- Complexity: ~80 LoC + scripts

### [P10.02] Compare overhead vs raw service calls
- Steps:
  - Plan Compare overhead vs raw service calls
  - Implement & tests
  - Document
- Estimate: 2-3h
- Scope: In scope: Compare overhead vs raw service calls. Out of scope: unrelated transports/services.
- Complexity: ~80 LoC

### [P10.03] Profile hot paths (event emission, state checks)
- Steps:
  - Plan Profile hot paths (event emission, state checks)
  - Implement & tests
  - Document
- Estimate: 3-5h
- Scope: In scope: Profile hot paths (event emission, state checks). Out of scope: unrelated transports/services.
- Complexity: ~80 LoC + scripts

### [P10.04] Optimize lock contention
- Steps:
  - Plan Optimize lock contention
  - Implement & tests
  - Document
- Estimate: 2-3h
- Scope: In scope: Optimize lock contention. Out of scope: unrelated transports/services.
- Complexity: ~80 LoC

### [P10.05] < 1% latency overhead for policy layers
- Steps:
  - Plan < 1% latency overhead for policy layers
  - Implement & tests
  - Document
- Estimate: 2-3h
- Scope: In scope: < 1% latency overhead for policy layers. Out of scope: unrelated transports/services.
- Complexity: ~80 LoC

### [P10.06] < 10μs per event emission
- Steps:
  - Plan < 10μs per event emission
  - Implement & tests
  - Document
- Estimate: 2-3h
- Scope: In scope: < 10μs per event emission. Out of scope: unrelated transports/services.
- Complexity: ~80 LoC

### [P10.07] Lock-free fast paths where possible
- Steps:
  - Plan Lock-free fast paths where possible
  - Implement & tests
  - Document
- Estimate: 2-3h
- Scope: In scope: Lock-free fast paths where possible. Out of scope: unrelated transports/services.
- Complexity: ~80 LoC

### [P10.08] Chaos engineering tests
- Steps:
  - Plan Chaos engineering tests
  - Implement & tests
  - Document
- Estimate: 2-3h
- Scope: In scope: Chaos engineering tests. Out of scope: unrelated transports/services.
- Complexity: ~80 LoC

### [P10.09] Soak tests (run for days)
- Steps:
  - Plan Soak tests (run for days)
  - Implement & tests
  - Document
- Estimate: 2-3h
- Scope: In scope: Soak tests (run for days). Out of scope: unrelated transports/services.
- Complexity: ~80 LoC

### [P10.10] Load tests (thousands of RPS)
- Steps:
  - Plan Load tests (thousands of RPS)
  - Implement & tests
  - Document
- Estimate: 3-5h
- Scope: In scope: Load tests (thousands of RPS). Out of scope: unrelated transports/services.
- Complexity: ~80 LoC + scripts

### [P10.11] Failure injection tests
- Steps:
  - Plan Failure injection tests
  - Implement & tests
  - Document
- Estimate: 2-3h
- Scope: In scope: Failure injection tests. Out of scope: unrelated transports/services.
- Complexity: ~80 LoC

### [P10.12] Add detailed tracing spans to all layers
- Steps:
  - Plan Add detailed tracing spans to all layers
  - Implement & tests
  - Document
- Estimate: 2-4h
- Scope: In scope: Add detailed tracing spans to all layers. Out of scope: unrelated transports/services.
- Complexity: ~100 LoC

### [P10.13] Metrics integration (Prometheus)
- Steps:
  - Plan Metrics integration (Prometheus)
  - Implement & tests
  - Document
- Estimate: 2-3h
- Scope: In scope: Metrics integration (Prometheus). Out of scope: unrelated transports/services.
- Complexity: ~80 LoC

### [P10.14] Add flame graph generation support
- Steps:
  - Plan Add flame graph generation support
  - Implement & tests
  - Document
- Estimate: 2-4h
- Scope: In scope: Add flame graph generation support. Out of scope: unrelated transports/services.
- Complexity: ~100 LoC
