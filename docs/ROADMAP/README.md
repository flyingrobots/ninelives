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

### [P0.01.a] timeout.rs -> TimeoutLayer + TimeoutService
- Steps:
  - Plan timeout.rs -> TimeoutLayer + TimeoutService
  - Implement core
- Estimate: ~2h
- Scope: In scope: timeout.rs -> TimeoutLayer + TimeoutService. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P0.01.b] timeout.rs -> TimeoutLayer + TimeoutService
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: timeout.rs -> TimeoutLayer + TimeoutService. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P0.02.a] retry.rs -> RetryLayer + RetryService
- Steps:
  - Plan retry.rs -> RetryLayer + RetryService
  - Implement core
- Estimate: ~2h
- Scope: In scope: retry.rs -> RetryLayer + RetryService. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P0.02.b] retry.rs -> RetryLayer + RetryService
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: retry.rs -> RetryLayer + RetryService. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P0.03.a] bulkhead.rs -> BulkheadLayer + BulkheadService
- Steps:
  - Plan bulkhead.rs -> BulkheadLayer + BulkheadService
  - Implement core
- Estimate: ~2h
- Scope: In scope: bulkhead.rs -> BulkheadLayer + BulkheadService. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P0.03.b] bulkhead.rs -> BulkheadLayer + BulkheadService
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: bulkhead.rs -> BulkheadLayer + BulkheadService. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P0.04.a] circuit_breaker.rs -> CircuitBreakerLayer + CircuitBreakerService
- Steps:
  - Plan circuit_breaker.rs -> CircuitBreakerLayer + CircuitBreakerService
  - Implement core
- Estimate: ~2h
- Scope: In scope: circuit_breaker.rs -> CircuitBreakerLayer + CircuitBreakerService. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P0.04.b] circuit_breaker.rs -> CircuitBreakerLayer + CircuitBreakerService
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: circuit_breaker.rs -> CircuitBreakerLayer + CircuitBreakerService. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P0.05.a] Backoff/Jitter integration with retry layer
- Steps:
  - Plan Backoff/Jitter integration with retry layer
  - Implement core
- Estimate: ~2h
- Scope: In scope: Backoff/Jitter integration with retry layer. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P0.05.b] Backoff/Jitter integration with retry layer
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Backoff/Jitter integration with retry layer. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P0.06.a] Policy wrapper
- Steps:
  - Plan Policy wrapper
  - Implement core
- Estimate: ~2h
- Scope: In scope: Policy wrapper. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P0.06.b] Policy wrapper
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Policy wrapper. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P0.07.a] `+` operator (CombinedLayer for sequential composition)
- Steps:
  - Plan `+` operator (CombinedLayer for sequential composition)
  - Implement core
- Estimate: ~2h
- Scope: In scope: `+` operator (CombinedLayer for sequential composition). Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P0.07.b] `+` operator (CombinedLayer for sequential composition)
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: `+` operator (CombinedLayer for sequential composition). Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P0.08.a] `|` operator (FallbackLayer for failover)
- Steps:
  - Plan `|` operator (FallbackLayer for failover)
  - Implement core
- Estimate: ~2h
- Scope: In scope: `|` operator (FallbackLayer for failover). Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P0.08.b] `|` operator (FallbackLayer for failover)
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: `|` operator (FallbackLayer for failover). Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P0.09] Doc comments and examples for algebra API
- Steps:
  - Plan Doc comments and examples for algebra API
  - Implement & tests
  - Document
- Estimate: 1-2h
- Scope: In scope: Doc comments and examples for algebra API. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P0.10] Operator precedence documentation
- Steps:
  - Plan Operator precedence documentation
  - Implement & tests
  - Document
- Estimate: 1-2h
- Scope: In scope: Operator precedence documentation. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P0.11.a] Delete src/stack.rs
- Steps:
  - Plan Delete src/stack.rs
  - Implement core
- Estimate: ~2h
- Scope: In scope: Delete src/stack.rs. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P0.11.b] Delete src/stack.rs
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Delete src/stack.rs. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P0.12.a] Remove ResilienceStack exports from lib/prelude
- Steps:
  - Plan Remove ResilienceStack exports from lib/prelude
  - Implement core
- Estimate: ~2h
- Scope: In scope: Remove ResilienceStack exports from lib/prelude. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P0.12.b] Remove ResilienceStack exports from lib/prelude
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Remove ResilienceStack exports from lib/prelude. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P0.13.a] Delete examples/full_stack.rs
- Steps:
  - Plan Delete examples/full_stack.rs
  - Implement core
- Estimate: ~2h
- Scope: In scope: Delete examples/full_stack.rs. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P0.13.b] Delete examples/full_stack.rs
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Delete examples/full_stack.rs. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P0.14.a] Remove legacy references from tests
- Steps:
  - Plan Remove legacy references from tests
  - Implement core
- Estimate: ~2h
- Scope: In scope: Remove legacy references from tests. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P0.14.b] Remove legacy references from tests
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Remove legacy references from tests. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P0.15.a] Update lib.rs with new quick start (Policy + tower ServiceBuilder)
- Steps:
  - Plan Update lib.rs with new quick start (Policy + tower ServiceBuilder)
  - Implement core
- Estimate: ~2h
- Scope: In scope: Update lib.rs with new quick start (Policy + tower ServiceBuilder). Out of scope: unrelated transports/services.
- Complexity: ~50 LoC

### [P0.15.b] Update lib.rs with new quick start (Policy + tower ServiceBuilder)
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Update lib.rs with new quick start (Policy + tower ServiceBuilder). Out of scope: unrelated transports/services.
- Complexity: ~50 LoC

### [P0.16.a] Update prelude.rs with algebra re-exports
- Steps:
  - Plan Update prelude.rs with algebra re-exports
  - Implement core
- Estimate: ~2h
- Scope: In scope: Update prelude.rs with algebra re-exports. Out of scope: unrelated transports/services.
- Complexity: ~50 LoC

### [P0.16.b] Update prelude.rs with algebra re-exports
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Update prelude.rs with algebra re-exports. Out of scope: unrelated transports/services.
- Complexity: ~50 LoC

### [P0.17.a] Create README.md with:
- Steps:
  - Plan Create README.md with:
  - Implement core
- Estimate: ~2h
- Scope: In scope: Create README.md with:. Out of scope: unrelated transports/services.
- Complexity: ~50 LoC

### [P0.17.b] Create README.md with:
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Create README.md with:. Out of scope: unrelated transports/services.
- Complexity: ~50 LoC

### [P0.18.a] Update examples/:
- Steps:
  - Plan Update examples/:
  - Implement core
- Estimate: ~2h
- Scope: In scope: Update examples/:. Out of scope: unrelated transports/services.
- Complexity: ~50 LoC

### [P0.18.b] Update examples/:
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Update examples/:. Out of scope: unrelated transports/services.
- Complexity: ~50 LoC

### [P0.19] Add doc tests for algebra operators
- Steps:
  - Plan Add doc tests for algebra operators
  - Implement & tests
  - Document
- Estimate: 1-2h
- Scope: In scope: Add doc tests for algebra operators. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P0.20.a] Adapt integration tests to Layer/Service architecture
- Steps:
  - Plan Adapt integration tests to Layer/Service architecture
  - Implement core
- Estimate: ~2h
- Scope: In scope: Adapt integration tests to Layer/Service architecture. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P0.20.b] Adapt integration tests to Layer/Service architecture
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Adapt integration tests to Layer/Service architecture. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P0.21.a] Add test coverage for:
- Steps:
  - Plan Add test coverage for:
  - Implement core
- Estimate: ~2h
- Scope: In scope: Add test coverage for:. Out of scope: unrelated transports/services.
- Complexity: ~50 LoC

### [P0.21.b] Add test coverage for:
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Add test coverage for:. Out of scope: unrelated transports/services.
- Complexity: ~50 LoC

### [P0.22.a] Ensure clippy passes
- Steps:
  - Plan Ensure clippy passes
  - Implement core
- Estimate: ~2h
- Scope: In scope: Ensure clippy passes. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P0.22.b] Ensure clippy passes
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Ensure clippy passes. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P0.23] Ensure all doc tests pass
- Steps:
  - Plan Ensure all doc tests pass
  - Implement & tests
  - Document
- Estimate: 1-2h
- Scope: In scope: Ensure all doc tests pass. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P0.24.a] Add CI workflow if missing
- Steps:
  - Plan Add CI workflow if missing
  - Implement core
- Estimate: ~2h
- Scope: In scope: Add CI workflow if missing. Out of scope: unrelated transports/services.
- Complexity: ~50 LoC

### [P0.24.b] Add CI workflow if missing
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Add CI workflow if missing. Out of scope: unrelated transports/services.
- Complexity: ~50 LoC

### [P1.01.a] Define `PolicyEvent` enum:
- Steps:
  - Plan Define `PolicyEvent` enum:
  - Implement core
- Estimate: ~2h
- Scope: In scope: Define `PolicyEvent` enum:. Out of scope: unrelated transports/services.
- Complexity: ~60 LoC

### [P1.01.b] Define `PolicyEvent` enum:
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Define `PolicyEvent` enum:. Out of scope: unrelated transports/services.
- Complexity: ~60 LoC

### [P1.02.a] Add event emission to all policy layers:
- Steps:
  - Plan Add event emission to all policy layers:
  - Implement core
- Estimate: ~2h
- Scope: In scope: Add event emission to all policy layers:. Out of scope: unrelated transports/services.
- Complexity: ~50 LoC

### [P1.02.b] Add event emission to all policy layers:
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Add event emission to all policy layers:. Out of scope: unrelated transports/services.
- Complexity: ~50 LoC

### [P1.03.a] Define `TelemetrySink` as a `tower::Service<PolicyEvent, Response=(), Error=E>`
- Steps:
  - Plan Define `TelemetrySink` as a `tower::Service<PolicyEvent, Response=(), Error=E>`
  - Implement core
- Estimate: ~2h
- Scope: In scope: Define `TelemetrySink` as a `tower::Service<PolicyEvent, Response=(), Error=E>`. Out of scope: unrelated transports/services.
- Complexity: ~60 LoC

### [P1.03.b] Define `TelemetrySink` as a `tower::Service<PolicyEvent, Response=(), Error=E>`
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Define `TelemetrySink` as a `tower::Service<PolicyEvent, Response=(), Error=E>`. Out of scope: unrelated transports/services.
- Complexity: ~60 LoC

### [P1.04.a] Implement basic sinks:
- Steps:
  - Plan Implement basic sinks:
  - Implement core
- Estimate: ~2h
- Scope: In scope: Implement basic sinks:. Out of scope: unrelated transports/services.
- Complexity: ~50 LoC

### [P1.04.b] Implement basic sinks:
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Implement basic sinks:. Out of scope: unrelated transports/services.
- Complexity: ~50 LoC

### [P1.05.a] Wire policies to accept telemetry sink via `.with_sink()` method
- Steps:
  - Plan Wire policies to accept telemetry sink via `.with_sink()` method
  - Implement core
- Estimate: ~2h
- Scope: In scope: Wire policies to accept telemetry sink via `.with_sink()` method. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P1.05.b] Wire policies to accept telemetry sink via `.with_sink()` method
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Wire policies to accept telemetry sink via `.with_sink()` method. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P1.06.a] Implement `MulticastSink` for sending to multiple sinks (multicast to both)
- Steps:
  - Plan Implement `MulticastSink` for sending to multiple sinks (multicast to both)
  - Implement core
- Estimate: ~2h
- Scope: In scope: Implement `MulticastSink` for sending to multiple sinks (multicast to both). Out of scope: unrelated transports/services.
- Complexity: ~50 LoC

### [P1.06.b] Implement `MulticastSink` for sending to multiple sinks (multicast to both)
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Implement `MulticastSink` for sending to multiple sinks (multicast to both). Out of scope: unrelated transports/services.
- Complexity: ~50 LoC

### [P1.07.a] Implement `FallbackSink` for fallback on failure
- Steps:
  - Plan Implement `FallbackSink` for fallback on failure
  - Implement core
- Estimate: ~2h
- Scope: In scope: Implement `FallbackSink` for fallback on failure. Out of scope: unrelated transports/services.
- Complexity: ~50 LoC

### [P1.07.b] Implement `FallbackSink` for fallback on failure
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Implement `FallbackSink` for fallback on failure. Out of scope: unrelated transports/services.
- Complexity: ~50 LoC

### [P1.08.a] Add `ComposedSinkError` type for composition errors
- Steps:
  - Plan Add `ComposedSinkError` type for composition errors
  - Implement core
- Estimate: ~2h
- Scope: In scope: Add `ComposedSinkError` type for composition errors. Out of scope: unrelated transports/services.
- Complexity: ~50 LoC

### [P1.08.b] Add `ComposedSinkError` type for composition errors
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Add `ComposedSinkError` type for composition errors. Out of scope: unrelated transports/services.
- Complexity: ~50 LoC

### [P1.09] Document sink composition patterns
- Steps:
  - Plan Document sink composition patterns
  - Implement & tests
  - Document
- Estimate: 1-2h
- Scope: In scope: Document sink composition patterns. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P1.10.a] Thread sink through policy constructors/builders via `.with_sink()` method
- Steps:
  - Plan Thread sink through policy constructors/builders via `.with_sink()` method
  - Implement core
- Estimate: ~2h
- Scope: In scope: Thread sink through policy constructors/builders via `.with_sink()` method. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P1.10.b] Thread sink through policy constructors/builders via `.with_sink()` method
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Thread sink through policy constructors/builders via `.with_sink()` method. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P1.11.a] Add examples showing telemetry integration:
- Steps:
  - Plan Add examples showing telemetry integration:
  - Implement core
- Estimate: ~2h
- Scope: In scope: Add examples showing telemetry integration:. Out of scope: unrelated transports/services.
- Complexity: ~50 LoC

### [P1.11.b] Add examples showing telemetry integration:
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Add examples showing telemetry integration:. Out of scope: unrelated transports/services.
- Complexity: ~50 LoC

### [P1.12.a] Benchmark overhead of event emission (deferred to Phase 10)
- Steps:
  - Plan Benchmark overhead of event emission (deferred to Phase 10)
  - Implement core
- Estimate: ~2h
- Scope: In scope: Benchmark overhead of event emission (deferred to Phase 10). Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P1.12.b] Benchmark overhead of event emission (deferred to Phase 10)
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Benchmark overhead of event emission (deferred to Phase 10). Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P2.01.a] Design `Adaptive<T>` wrapper:
- Steps:
  - Plan Design `Adaptive<T>` wrapper:
  - Implement core
- Estimate: ~2h
- Scope: In scope: Design `Adaptive<T>` wrapper:. Out of scope: unrelated transports/services.
- Complexity: ~60 LoC

### [P2.01.b] Design `Adaptive<T>` wrapper:
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Design `Adaptive<T>` wrapper:. Out of scope: unrelated transports/services.
- Complexity: ~60 LoC

### [P2.02.a] Integrate Adaptive into policy configs:
- Steps:
  - Plan Integrate Adaptive into policy configs:
  - Implement core
- Estimate: ~2h
- Scope: In scope: Integrate Adaptive into policy configs:. Out of scope: unrelated transports/services.
- Complexity: ~50 LoC

### [P2.02.b] Integrate Adaptive into policy configs:
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Integrate Adaptive into policy configs:. Out of scope: unrelated transports/services.
- Complexity: ~50 LoC

### [P2.03.a] Define `CommandContext` struct:
- Steps:
  - Plan Define `CommandContext` struct:
  - Implement core
- Estimate: ~2h
- Scope: In scope: Define `CommandContext` struct:. Out of scope: unrelated transports/services.
- Complexity: ~60 LoC

### [P2.03.b] Define `CommandContext` struct:
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Define `CommandContext` struct:. Out of scope: unrelated transports/services.
- Complexity: ~60 LoC

### [P2.04.a] Define `CommandHandler` trait as `tower::Service<CommandContext>`
- Steps:
  - Plan Define `CommandHandler` trait as `tower::Service<CommandContext>`
  - Implement core
- Estimate: ~2h
- Scope: In scope: Define `CommandHandler` trait as `tower::Service<CommandContext>`. Out of scope: unrelated transports/services.
- Complexity: ~60 LoC

### [P2.04.b] Define `CommandHandler` trait as `tower::Service<CommandContext>`
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Define `CommandHandler` trait as `tower::Service<CommandContext>`. Out of scope: unrelated transports/services.
- Complexity: ~60 LoC

### [P2.05.a] Implement `ControlPlaneRouter`:
- Steps:
  - Plan Implement `ControlPlaneRouter`:
  - Implement core
- Estimate: ~2h
- Scope: In scope: Implement `ControlPlaneRouter`:. Out of scope: unrelated transports/services.
- Complexity: ~50 LoC

### [P2.05.b] Implement `ControlPlaneRouter`:
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Implement `ControlPlaneRouter`:. Out of scope: unrelated transports/services.
- Complexity: ~50 LoC

### [P2.06.a] `SetParameterHandler` (update Adaptive values)
- Steps:
  - Plan `SetParameterHandler` (update Adaptive values)
  - Implement core
- Estimate: ~2h
- Scope: In scope: `SetParameterHandler` (update Adaptive values). Out of scope: unrelated transports/services.
- Complexity: ~50 LoC

### [P2.06.b] `SetParameterHandler` (update Adaptive values)
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: `SetParameterHandler` (update Adaptive values). Out of scope: unrelated transports/services.
- Complexity: ~50 LoC

### [P2.07.a] `GetParameterHandler` (read current config)
- Steps:
  - Plan `GetParameterHandler` (read current config)
  - Implement core
- Estimate: ~2h
- Scope: In scope: `GetParameterHandler` (read current config). Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P2.07.b] `GetParameterHandler` (read current config)
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: `GetParameterHandler` (read current config). Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P2.08.a] `GetStateHandler` (query policy state)
- Steps:
  - Plan `GetStateHandler` (query policy state)
  - Implement core
- Estimate: ~2h
- Scope: In scope: `GetStateHandler` (query policy state). Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P2.08.b] `GetStateHandler` (query policy state)
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: `GetStateHandler` (query policy state). Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P2.09.a] `ResetCircuitBreakerHandler`
- Steps:
  - Plan `ResetCircuitBreakerHandler`
  - Implement core
- Estimate: ~2h
- Scope: In scope: `ResetCircuitBreakerHandler`. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P2.09.b] `ResetCircuitBreakerHandler`
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: `ResetCircuitBreakerHandler`. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P2.10.a] `ListPoliciesHandler`
- Steps:
  - Plan `ListPoliciesHandler`
  - Implement core
- Estimate: ~2h
- Scope: In scope: `ListPoliciesHandler`. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P2.10.b] `ListPoliciesHandler`
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: `ListPoliciesHandler`. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P2.11.a] Start with `ninelives-control` crate
- Steps:
  - Plan Start with `ninelives-control` crate
  - Implement core
- Estimate: ~2h
- Scope: In scope: Start with `ninelives-control` crate. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P2.11.b] Start with `ninelives-control` crate
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Start with `ninelives-control` crate. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P2.12.a] Implement local/in-process transport (channels)
- Steps:
  - Plan Implement local/in-process transport (channels)
  - Implement core
- Estimate: ~2h
- Scope: In scope: Implement local/in-process transport (channels). Out of scope: unrelated transports/services.
- Complexity: ~50 LoC

### [P2.12.b] Implement local/in-process transport (channels)
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Implement local/in-process transport (channels). Out of scope: unrelated transports/services.
- Complexity: ~50 LoC

### [P2.13.a] Design transport abstraction for future HTTP/gRPC/etc.
- Steps:
  - Plan Design transport abstraction for future HTTP/gRPC/etc.
  - Implement core
- Estimate: ~2h
- Scope: In scope: Design transport abstraction for future HTTP/gRPC/etc.. Out of scope: unrelated transports/services.
- Complexity: ~60 LoC

### [P2.13.b] Design transport abstraction for future HTTP/gRPC/etc.
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Design transport abstraction for future HTTP/gRPC/etc.. Out of scope: unrelated transports/services.
- Complexity: ~60 LoC

### [P2.14.a] Define `AuthorizationLayer` (checks Identity in CommandContext)
- Steps:
  - Plan Define `AuthorizationLayer` (checks Identity in CommandContext)
  - Implement core
- Estimate: ~2h
- Scope: In scope: Define `AuthorizationLayer` (checks Identity in CommandContext). Out of scope: unrelated transports/services.
- Complexity: ~60 LoC

### [P2.14.b] Define `AuthorizationLayer` (checks Identity in CommandContext)
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Define `AuthorizationLayer` (checks Identity in CommandContext). Out of scope: unrelated transports/services.
- Complexity: ~60 LoC

### [P2.15.a] Define `AuditLayer` (logs all commands)
- Steps:
  - Plan Define `AuditLayer` (logs all commands)
  - Implement core
- Estimate: ~2h
- Scope: In scope: Define `AuditLayer` (logs all commands). Out of scope: unrelated transports/services.
- Complexity: ~60 LoC

### [P2.15.b] Define `AuditLayer` (logs all commands)
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Define `AuditLayer` (logs all commands). Out of scope: unrelated transports/services.
- Complexity: ~60 LoC

### [P2.16.a] Wrap ControlPlaneRouter in Policy(AuthZ) + Policy(Audit)
- Steps:
  - Plan Wrap ControlPlaneRouter in Policy(AuthZ) + Policy(Audit)
  - Implement core
- Estimate: ~2h
- Scope: In scope: Wrap ControlPlaneRouter in Policy(AuthZ) + Policy(Audit). Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P2.16.b] Wrap ControlPlaneRouter in Policy(AuthZ) + Policy(Audit)
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Wrap ControlPlaneRouter in Policy(AuthZ) + Policy(Audit). Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P3.01.a] Design `SystemState` struct:
- Steps:
  - Plan Design `SystemState` struct:
  - Implement core
- Estimate: ~2h
- Scope: In scope: Design `SystemState` struct:. Out of scope: unrelated transports/services.
- Complexity: ~60 LoC

### [P3.01.b] Design `SystemState` struct:
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Design `SystemState` struct:. Out of scope: unrelated transports/services.
- Complexity: ~60 LoC

### [P3.02.a] Implement efficient storage (ring buffers, sketches)
- Steps:
  - Plan Implement efficient storage (ring buffers, sketches)
  - Implement core
- Estimate: ~2h
- Scope: In scope: Implement efficient storage (ring buffers, sketches). Out of scope: unrelated transports/services.
- Complexity: ~50 LoC

### [P3.02.b] Implement efficient storage (ring buffers, sketches)
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Implement efficient storage (ring buffers, sketches). Out of scope: unrelated transports/services.
- Complexity: ~50 LoC

### [P3.03.a] Create `ninelives-observer` crate
- Steps:
  - Plan Create `ninelives-observer` crate
  - Implement core
- Estimate: ~2h
- Scope: In scope: Create `ninelives-observer` crate. Out of scope: unrelated transports/services.
- Complexity: ~50 LoC

### [P3.03.b] Create `ninelives-observer` crate
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Create `ninelives-observer` crate. Out of scope: unrelated transports/services.
- Complexity: ~50 LoC

### [P3.04.a] Implement `Observer` as a background task
- Steps:
  - Plan Implement `Observer` as a background task
  - Implement core
- Estimate: ~2h
- Scope: In scope: Implement `Observer` as a background task. Out of scope: unrelated transports/services.
- Complexity: ~50 LoC

### [P3.04.b] Implement `Observer` as a background task
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Implement `Observer` as a background task. Out of scope: unrelated transports/services.
- Complexity: ~50 LoC

### [P3.05.a] Subscribe to StreamingSink
- Steps:
  - Plan Subscribe to StreamingSink
  - Implement core
- Estimate: ~2h
- Scope: In scope: Subscribe to StreamingSink. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P3.05.b] Subscribe to StreamingSink
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Subscribe to StreamingSink. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P3.06.a] Ingest PolicyEvents and update SystemState
- Steps:
  - Plan Ingest PolicyEvents and update SystemState
  - Implement core
- Estimate: ~2h
- Scope: In scope: Ingest PolicyEvents and update SystemState. Out of scope: unrelated transports/services.
- Complexity: ~50 LoC

### [P3.06.b] Ingest PolicyEvents and update SystemState
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Ingest PolicyEvents and update SystemState. Out of scope: unrelated transports/services.
- Complexity: ~50 LoC

### [P3.07.a] Expose query interface:
- Steps:
  - Plan Expose query interface:
  - Implement core
- Estimate: ~2h
- Scope: In scope: Expose query interface:. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P3.07.b] Expose query interface:
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Expose query interface:. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P3.08.a] Wire Observer to telemetry message bus
- Steps:
  - Plan Wire Observer to telemetry message bus
  - Implement core
- Estimate: ~2h
- Scope: In scope: Wire Observer to telemetry message bus. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P3.08.b] Wire Observer to telemetry message bus
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Wire Observer to telemetry message bus. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P3.09.a] Add control plane commands to query Observer state
- Steps:
  - Plan Add control plane commands to query Observer state
  - Implement core
- Estimate: ~2h
- Scope: In scope: Add control plane commands to query Observer state. Out of scope: unrelated transports/services.
- Complexity: ~50 LoC

### [P3.09.b] Add control plane commands to query Observer state
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Add control plane commands to query Observer state. Out of scope: unrelated transports/services.
- Complexity: ~50 LoC

### [P3.10.a] Add examples showing Observer usage
- Steps:
  - Plan Add examples showing Observer usage
  - Implement core
- Estimate: ~2h
- Scope: In scope: Add examples showing Observer usage. Out of scope: unrelated transports/services.
- Complexity: ~50 LoC

### [P3.10.b] Add examples showing Observer usage
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Add examples showing Observer usage. Out of scope: unrelated transports/services.
- Complexity: ~50 LoC

### [P4.01.a] Design `ForkJoinLayer` and `ForkJoinService`
- Steps:
  - Plan Design `ForkJoinLayer` and `ForkJoinService`
  - Implement core
- Estimate: ~2h
- Scope: In scope: Design `ForkJoinLayer` and `ForkJoinService`. Out of scope: unrelated transports/services.
- Complexity: ~60 LoC

### [P4.01.b] Design `ForkJoinLayer` and `ForkJoinService`
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Design `ForkJoinLayer` and `ForkJoinService`. Out of scope: unrelated transports/services.
- Complexity: ~60 LoC

### [P4.02.a] Spawn both services concurrently (futures::select for racing)
- Steps:
  - Plan Spawn both services concurrently (futures::select for racing)
  - Implement core
- Estimate: ~2h
- Scope: In scope: Spawn both services concurrently (futures::select for racing). Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P4.02.b] Spawn both services concurrently (futures::select for racing)
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Spawn both services concurrently (futures::select for racing). Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P4.03.a] Return first `Ok` result
- Steps:
  - Plan Return first `Ok` result
  - Implement core
- Estimate: ~2h
- Scope: In scope: Return first `Ok` result. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P4.03.b] Return first `Ok` result
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Return first `Ok` result. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P4.04.a] Cancel remaining futures on first success
- Steps:
  - Plan Cancel remaining futures on first success
  - Implement core
- Estimate: ~2h
- Scope: In scope: Cancel remaining futures on first success. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P4.04.b] Cancel remaining futures on first success
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Cancel remaining futures on first success. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P4.05.a] Handle case where both fail (return error)
- Steps:
  - Plan Handle case where both fail (return error)
  - Implement core
- Estimate: ~2h
- Scope: In scope: Handle case where both fail (return error). Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P4.05.b] Handle case where both fail (return error)
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Handle case where both fail (return error). Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P4.06.a] Implement `BitAnd` trait for `Policy<L>`
- Steps:
  - Plan Implement `BitAnd` trait for `Policy<L>`
  - Implement core
- Estimate: ~2h
- Scope: In scope: Implement `BitAnd` trait for `Policy<L>`. Out of scope: unrelated transports/services.
- Complexity: ~50 LoC

### [P4.06.b] Implement `BitAnd` trait for `Policy<L>`
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Implement `BitAnd` trait for `Policy<L>`. Out of scope: unrelated transports/services.
- Complexity: ~50 LoC

### [P4.07.a] Returns `Policy<ForkJoinLayer<A, B>>`
- Steps:
  - Plan Returns `Policy<ForkJoinLayer<A, B>>`
  - Implement core
- Estimate: ~2h
- Scope: In scope: Returns `Policy<ForkJoinLayer<A, B>>`. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P4.07.b] Returns `Policy<ForkJoinLayer<A, B>>`
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Returns `Policy<ForkJoinLayer<A, B>>`. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P4.08] Test race conditions (doc tests cover both sides)
- Steps:
  - Plan Test race conditions (doc tests cover both sides)
  - Implement & tests
  - Document
- Estimate: 1-2h
- Scope: In scope: Test race conditions (doc tests cover both sides). Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P4.09.a] Test both-fail scenarios (implemented in service logic)
- Steps:
  - Plan Test both-fail scenarios (implemented in service logic)
  - Implement core
- Estimate: ~2h
- Scope: In scope: Test both-fail scenarios (implemented in service logic). Out of scope: unrelated transports/services.
- Complexity: ~50 LoC

### [P4.09.b] Test both-fail scenarios (implemented in service logic)
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Test both-fail scenarios (implemented in service logic). Out of scope: unrelated transports/services.
- Complexity: ~50 LoC

### [P4.10.a] Test cancellation behavior (futures::select handles drop)
- Steps:
  - Plan Test cancellation behavior (futures::select handles drop)
  - Implement core
- Estimate: ~2h
- Scope: In scope: Test cancellation behavior (futures::select handles drop). Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P4.10.b] Test cancellation behavior (futures::select handles drop)
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Test cancellation behavior (futures::select handles drop). Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P4.11.a] Benchmark overhead vs sequential
- Steps:
  - Plan Benchmark overhead vs sequential
  - Implement core
- Estimate: ~2h
- Scope: In scope: Benchmark overhead vs sequential. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P4.11.b] Benchmark overhead vs sequential
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Benchmark overhead vs sequential. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P4.12.a] Add examples: IPv4/IPv6, cache strategies
- Steps:
  - Plan Add examples: IPv4/IPv6, cache strategies
  - Implement core
- Estimate: ~2h
- Scope: In scope: Add examples: IPv4/IPv6, cache strategies. Out of scope: unrelated transports/services.
- Complexity: ~50 LoC

### [P4.12.b] Add examples: IPv4/IPv6, cache strategies
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Add examples: IPv4/IPv6, cache strategies. Out of scope: unrelated transports/services.
- Complexity: ~50 LoC

### [P4.13] Document operator precedence: `&` > `+` > `|`
- Steps:
  - Plan Document operator precedence: `&` > `+` > `|`
  - Implement & tests
  - Document
- Estimate: 1-2h
- Scope: In scope: Document operator precedence: `&` > `+` > `|`. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P4.14.a] Add to algebra guide (README, lib.rs, examples)
- Steps:
  - Plan Add to algebra guide (README, lib.rs, examples)
  - Implement core
- Estimate: ~2h
- Scope: In scope: Add to algebra guide (README, lib.rs, examples). Out of scope: unrelated transports/services.
- Complexity: ~50 LoC

### [P4.14.b] Add to algebra guide (README, lib.rs, examples)
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Add to algebra guide (README, lib.rs, examples). Out of scope: unrelated transports/services.
- Complexity: ~50 LoC

### [P5.01.a] Create `ninelives-sentinel` crate
- Steps:
  - Plan Create `ninelives-sentinel` crate
  - Implement core
- Estimate: ~2h
- Scope: In scope: Create `ninelives-sentinel` crate. Out of scope: unrelated transports/services.
- Complexity: ~50 LoC

### [P5.01.b] Create `ninelives-sentinel` crate
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Create `ninelives-sentinel` crate. Out of scope: unrelated transports/services.
- Complexity: ~50 LoC

### [P5.02.a] Integrate Rhai scripting engine
- Steps:
  - Plan Integrate Rhai scripting engine
  - Implement core
- Estimate: ~2h
- Scope: In scope: Integrate Rhai scripting engine. Out of scope: unrelated transports/services.
- Complexity: ~50 LoC

### [P5.02.b] Integrate Rhai scripting engine
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Integrate Rhai scripting engine. Out of scope: unrelated transports/services.
- Complexity: ~50 LoC

### [P5.03.a] Define script API:
- Steps:
  - Plan Define script API:
  - Implement core
- Estimate: ~2h
- Scope: In scope: Define script API:. Out of scope: unrelated transports/services.
- Complexity: ~60 LoC

### [P5.03.b] Define script API:
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Define script API:. Out of scope: unrelated transports/services.
- Complexity: ~60 LoC

### [P5.04.a] Implement meta-policy evaluation loop:
- Steps:
  - Plan Implement meta-policy evaluation loop:
  - Implement core
- Estimate: ~2h
- Scope: In scope: Implement meta-policy evaluation loop:. Out of scope: unrelated transports/services.
- Complexity: ~50 LoC

### [P5.04.b] Implement meta-policy evaluation loop:
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Implement meta-policy evaluation loop:. Out of scope: unrelated transports/services.
- Complexity: ~50 LoC

### [P5.05.a] `ReloadMetaPolicyHandler` command
- Steps:
  - Plan `ReloadMetaPolicyHandler` command
  - Implement core
- Estimate: ~2h
- Scope: In scope: `ReloadMetaPolicyHandler` command. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P5.05.b] `ReloadMetaPolicyHandler` command
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: `ReloadMetaPolicyHandler` command. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P5.06.a] Watch script file for changes (optional)
- Steps:
  - Plan Watch script file for changes (optional)
  - Implement core
- Estimate: ~2h
- Scope: In scope: Watch script file for changes (optional). Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P5.06.b] Watch script file for changes (optional)
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Watch script file for changes (optional). Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P5.07.a] Validate script before activating
- Steps:
  - Plan Validate script before activating
  - Implement core
- Estimate: ~2h
- Scope: In scope: Validate script before activating. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P5.07.b] Validate script before activating
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Validate script before activating. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P5.08.a] Auto-adjust retry backoff based on error rate
- Steps:
  - Plan Auto-adjust retry backoff based on error rate
  - Implement core
- Estimate: ~2h
- Scope: In scope: Auto-adjust retry backoff based on error rate. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P5.08.b] Auto-adjust retry backoff based on error rate
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Auto-adjust retry backoff based on error rate. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P5.09.a] Open circuit breaker on sustained failures
- Steps:
  - Plan Open circuit breaker on sustained failures
  - Implement core
- Estimate: ~2h
- Scope: In scope: Open circuit breaker on sustained failures. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P5.09.b] Open circuit breaker on sustained failures
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Open circuit breaker on sustained failures. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P5.10.a] Increase bulkhead capacity under load
- Steps:
  - Plan Increase bulkhead capacity under load
  - Implement core
- Estimate: ~2h
- Scope: In scope: Increase bulkhead capacity under load. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P5.10.b] Increase bulkhead capacity under load
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Increase bulkhead capacity under load. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P5.11.a] Alert on anomalies
- Steps:
  - Plan Alert on anomalies
  - Implement core
- Estimate: ~2h
- Scope: In scope: Alert on anomalies. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P5.11.b] Alert on anomalies
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Alert on anomalies. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P5.12.a] Implement `Sentinel` as top-level coordinator:
- Steps:
  - Plan Implement `Sentinel` as top-level coordinator:
  - Implement core
- Estimate: ~2h
- Scope: In scope: Implement `Sentinel` as top-level coordinator:. Out of scope: unrelated transports/services.
- Complexity: ~50 LoC

### [P5.12.b] Implement `Sentinel` as top-level coordinator:
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Implement `Sentinel` as top-level coordinator:. Out of scope: unrelated transports/services.
- Complexity: ~50 LoC

### [P5.13.a] Add graceful shutdown
- Steps:
  - Plan Add graceful shutdown
  - Implement core
- Estimate: ~2h
- Scope: In scope: Add graceful shutdown. Out of scope: unrelated transports/services.
- Complexity: ~50 LoC

### [P5.13.b] Add graceful shutdown
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Add graceful shutdown. Out of scope: unrelated transports/services.
- Complexity: ~50 LoC

### [P6.01.a] Extend Adaptive<T> to support shadow values:
- Steps:
  - Plan Extend Adaptive<T> to support shadow values:
  - Implement core
- Estimate: ~2h
- Scope: In scope: Extend Adaptive<T> to support shadow values:. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P6.01.b] Extend Adaptive<T> to support shadow values:
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Extend Adaptive<T> to support shadow values:. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P6.02.a] Add shadow mode to policy layers:
- Steps:
  - Plan Add shadow mode to policy layers:
  - Implement core
- Estimate: ~2h
- Scope: In scope: Add shadow mode to policy layers:. Out of scope: unrelated transports/services.
- Complexity: ~50 LoC

### [P6.02.b] Add shadow mode to policy layers:
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Add shadow mode to policy layers:. Out of scope: unrelated transports/services.
- Complexity: ~50 LoC

### [P6.03.a] Define `ShadowEvent` type (includes primary + shadow results)
- Steps:
  - Plan Define `ShadowEvent` type (includes primary + shadow results)
  - Implement core
- Estimate: ~2h
- Scope: In scope: Define `ShadowEvent` type (includes primary + shadow results). Out of scope: unrelated transports/services.
- Complexity: ~60 LoC

### [P6.03.b] Define `ShadowEvent` type (includes primary + shadow results)
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Define `ShadowEvent` type (includes primary + shadow results). Out of scope: unrelated transports/services.
- Complexity: ~60 LoC

### [P6.04.a] Add shadow event emission to policies
- Steps:
  - Plan Add shadow event emission to policies
  - Implement core
- Estimate: ~2h
- Scope: In scope: Add shadow event emission to policies. Out of scope: unrelated transports/services.
- Complexity: ~50 LoC

### [P6.04.b] Add shadow event emission to policies
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Add shadow event emission to policies. Out of scope: unrelated transports/services.
- Complexity: ~50 LoC

### [P6.05.a] Observer ingests shadow events separately
- Steps:
  - Plan Observer ingests shadow events separately
  - Implement core
- Estimate: ~2h
- Scope: In scope: Observer ingests shadow events separately. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P6.05.b] Observer ingests shadow events separately
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Observer ingests shadow events separately. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P6.06.a] Sentinel observes shadow stability over time window
- Steps:
  - Plan Sentinel observes shadow stability over time window
  - Implement core
- Estimate: ~2h
- Scope: In scope: Sentinel observes shadow stability over time window. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P6.06.b] Sentinel observes shadow stability over time window
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Sentinel observes shadow stability over time window. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P6.07.a] Issues `PromoteShadowHandler` command if stable
- Steps:
  - Plan Issues `PromoteShadowHandler` command if stable
  - Implement core
- Estimate: ~2h
- Scope: In scope: Issues `PromoteShadowHandler` command if stable. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P6.07.b] Issues `PromoteShadowHandler` command if stable
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Issues `PromoteShadowHandler` command if stable. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P6.08.a] Policies atomically swap shadow -> primary
- Steps:
  - Plan Policies atomically swap shadow -> primary
  - Implement core
- Estimate: ~2h
- Scope: In scope: Policies atomically swap shadow -> primary. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P6.08.b] Policies atomically swap shadow -> primary
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Policies atomically swap shadow -> primary. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P6.09.a] Ensure shadow evaluation doesn't affect primary path latency
- Steps:
  - Plan Ensure shadow evaluation doesn't affect primary path latency
  - Implement core
- Estimate: ~2h
- Scope: In scope: Ensure shadow evaluation doesn't affect primary path latency. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P6.09.b] Ensure shadow evaluation doesn't affect primary path latency
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Ensure shadow evaluation doesn't affect primary path latency. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P6.10.a] Add circuit breaker to kill shadow eval if too expensive
- Steps:
  - Plan Add circuit breaker to kill shadow eval if too expensive
  - Implement core
- Estimate: ~2h
- Scope: In scope: Add circuit breaker to kill shadow eval if too expensive. Out of scope: unrelated transports/services.
- Complexity: ~50 LoC

### [P6.10.b] Add circuit breaker to kill shadow eval if too expensive
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Add circuit breaker to kill shadow eval if too expensive. Out of scope: unrelated transports/services.
- Complexity: ~50 LoC

### [P6.11] Document safety guarantees
- Steps:
  - Plan Document safety guarantees
  - Implement & tests
  - Document
- Estimate: 1-2h
- Scope: In scope: Document safety guarantees. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P7.01.a] Create workspace Cargo.toml
- Steps:
  - Plan Create workspace Cargo.toml
  - Implement core
- Estimate: ~2h
- Scope: In scope: Create workspace Cargo.toml. Out of scope: unrelated transports/services.
- Complexity: ~50 LoC

### [P7.01.b] Create workspace Cargo.toml
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Create workspace Cargo.toml. Out of scope: unrelated transports/services.
- Complexity: ~50 LoC

### [P7.02.a] Split crates:
- Steps:
  - Plan Split crates:
  - Implement core
- Estimate: ~2h
- Scope: In scope: Split crates:. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P7.02.b] Split crates:
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Split crates:. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P7.03.a] Update dependencies and re-exports
- Steps:
  - Plan Update dependencies and re-exports
  - Implement core
- Estimate: ~2h
- Scope: In scope: Update dependencies and re-exports. Out of scope: unrelated transports/services.
- Complexity: ~50 LoC

### [P7.03.b] Update dependencies and re-exports
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Update dependencies and re-exports. Out of scope: unrelated transports/services.
- Complexity: ~50 LoC

### [P7.04.a] Ensure backward compatibility
- Steps:
  - Plan Ensure backward compatibility
  - Implement core
- Estimate: ~2h
- Scope: In scope: Ensure backward compatibility. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P7.04.b] Ensure backward compatibility
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Ensure backward compatibility. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P7.05.a] Create adapter template/guide
- Steps:
  - Plan Create adapter template/guide
  - Implement core
- Estimate: ~2h
- Scope: In scope: Create adapter template/guide. Out of scope: unrelated transports/services.
- Complexity: ~50 LoC

### [P7.05.b] Create adapter template/guide
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Create adapter template/guide. Out of scope: unrelated transports/services.
- Complexity: ~50 LoC

### [P7.06.a] Implement priority adapters:
- Steps:
  - Plan Implement priority adapters:
  - Implement core
- Estimate: ~2h
- Scope: In scope: Implement priority adapters:. Out of scope: unrelated transports/services.
- Complexity: ~50 LoC

### [P7.06.b] Implement priority adapters:
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Implement priority adapters:. Out of scope: unrelated transports/services.
- Complexity: ~50 LoC

### [P7.07] Document adapter development
- Steps:
  - Plan Document adapter development
  - Implement & tests
  - Document
- Estimate: 1-2h
- Scope: In scope: Document adapter development. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P8.01.a] Design transport-agnostic command serialization
- Steps:
  - Plan Design transport-agnostic command serialization
  - Implement core
- Estimate: ~2h
- Scope: In scope: Design transport-agnostic command serialization. Out of scope: unrelated transports/services.
- Complexity: ~60 LoC

### [P8.01.b] Design transport-agnostic command serialization
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Design transport-agnostic command serialization. Out of scope: unrelated transports/services.
- Complexity: ~60 LoC

### [P8.02.a] Support JSON and/or MessagePack
- Steps:
  - Plan Support JSON and/or MessagePack
  - Implement core
- Estimate: ~2h
- Scope: In scope: Support JSON and/or MessagePack. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P8.02.b] Support JSON and/or MessagePack
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Support JSON and/or MessagePack. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P8.03.a] Create `ninelives-rest` crate
- Steps:
  - Plan Create `ninelives-rest` crate
  - Implement core
- Estimate: ~2h
- Scope: In scope: Create `ninelives-rest` crate. Out of scope: unrelated transports/services.
- Complexity: ~50 LoC

### [P8.03.b] Create `ninelives-rest` crate
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Create `ninelives-rest` crate. Out of scope: unrelated transports/services.
- Complexity: ~50 LoC

### [P8.04.a] Expose ControlPlaneRouter over HTTP endpoints
- Steps:
  - Plan Expose ControlPlaneRouter over HTTP endpoints
  - Implement core
- Estimate: ~2h
- Scope: In scope: Expose ControlPlaneRouter over HTTP endpoints. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P8.04.b] Expose ControlPlaneRouter over HTTP endpoints
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Expose ControlPlaneRouter over HTTP endpoints. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P8.05.a] Add authentication middleware
- Steps:
  - Plan Add authentication middleware
  - Implement core
- Estimate: ~2h
- Scope: In scope: Add authentication middleware. Out of scope: unrelated transports/services.
- Complexity: ~50 LoC

### [P8.05.b] Add authentication middleware
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Add authentication middleware. Out of scope: unrelated transports/services.
- Complexity: ~50 LoC

### [P8.06.a] `ninelives-graphql` (GraphQL API)
- Steps:
  - Plan `ninelives-graphql` (GraphQL API)
  - Implement core
- Estimate: ~2h
- Scope: In scope: `ninelives-graphql` (GraphQL API). Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P8.06.b] `ninelives-graphql` (GraphQL API)
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: `ninelives-graphql` (GraphQL API). Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P8.07.a] `ninelives-mcp` (Model Context Protocol)
- Steps:
  - Plan `ninelives-mcp` (Model Context Protocol)
  - Implement core
- Estimate: ~2h
- Scope: In scope: `ninelives-mcp` (Model Context Protocol). Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P8.07.b] `ninelives-mcp` (Model Context Protocol)
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: `ninelives-mcp` (Model Context Protocol). Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P8.08.a] `ninelives-grpc` (gRPC service)
- Steps:
  - Plan `ninelives-grpc` (gRPC service)
  - Implement core
- Estimate: ~2h
- Scope: In scope: `ninelives-grpc` (gRPC service). Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P8.08.b] `ninelives-grpc` (gRPC service)
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: `ninelives-grpc` (gRPC service). Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P9.01.a] Autonomous Canary Releases
- Steps:
  - Plan Autonomous Canary Releases
  - Implement core
- Estimate: ~2h
- Scope: In scope: Autonomous Canary Releases. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P9.01.b] Autonomous Canary Releases
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Autonomous Canary Releases. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P9.02.a] Progressive Ratchet-Up
- Steps:
  - Plan Progressive Ratchet-Up
  - Implement core
- Estimate: ~2h
- Scope: In scope: Progressive Ratchet-Up. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P9.02.b] Progressive Ratchet-Up
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Progressive Ratchet-Up. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P9.03.a] Safety Valves (auto-scaling policies)
- Steps:
  - Plan Safety Valves (auto-scaling policies)
  - Implement core
- Estimate: ~2h
- Scope: In scope: Safety Valves (auto-scaling policies). Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P9.03.b] Safety Valves (auto-scaling policies)
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Safety Valves (auto-scaling policies). Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P9.04.a] Blue/Green Deployments
- Steps:
  - Plan Blue/Green Deployments
  - Implement core
- Estimate: ~2h
- Scope: In scope: Blue/Green Deployments. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P9.04.b] Blue/Green Deployments
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Blue/Green Deployments. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P9.05.a] Multi-Region Failover
- Steps:
  - Plan Multi-Region Failover
  - Implement core
- Estimate: ~2h
- Scope: In scope: Multi-Region Failover. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P9.05.b] Multi-Region Failover
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Multi-Region Failover. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P9.06.a] Build example apps in `examples/recipes/`
- Steps:
  - Plan Build example apps in `examples/recipes/`
  - Implement core
- Estimate: ~2h
- Scope: In scope: Build example apps in `examples/recipes/`. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P9.06.b] Build example apps in `examples/recipes/`
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Build example apps in `examples/recipes/`. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P9.07.a] Include Sentinel scripts for each pattern
- Steps:
  - Plan Include Sentinel scripts for each pattern
  - Implement core
- Estimate: ~2h
- Scope: In scope: Include Sentinel scripts for each pattern. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P9.07.b] Include Sentinel scripts for each pattern
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Include Sentinel scripts for each pattern. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P9.08.a] Add integration tests
- Steps:
  - Plan Add integration tests
  - Implement core
- Estimate: ~2h
- Scope: In scope: Add integration tests. Out of scope: unrelated transports/services.
- Complexity: ~50 LoC

### [P9.08.b] Add integration tests
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Add integration tests. Out of scope: unrelated transports/services.
- Complexity: ~50 LoC

### [P10.01.a] Criterion benchmarks for each layer
- Steps:
  - Plan Criterion benchmarks for each layer
  - Implement core
- Estimate: ~2h
- Scope: In scope: Criterion benchmarks for each layer. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P10.01.b] Criterion benchmarks for each layer
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Criterion benchmarks for each layer. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P10.02.a] Compare overhead vs raw service calls
- Steps:
  - Plan Compare overhead vs raw service calls
  - Implement core
- Estimate: ~2h
- Scope: In scope: Compare overhead vs raw service calls. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P10.02.b] Compare overhead vs raw service calls
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Compare overhead vs raw service calls. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P10.03.a] Profile hot paths (event emission, state checks)
- Steps:
  - Plan Profile hot paths (event emission, state checks)
  - Implement core
- Estimate: ~2h
- Scope: In scope: Profile hot paths (event emission, state checks). Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P10.03.b] Profile hot paths (event emission, state checks)
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Profile hot paths (event emission, state checks). Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P10.04.a] Optimize lock contention
- Steps:
  - Plan Optimize lock contention
  - Implement core
- Estimate: ~2h
- Scope: In scope: Optimize lock contention. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P10.04.b] Optimize lock contention
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Optimize lock contention. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P10.05.a] < 1% latency overhead for policy layers
- Steps:
  - Plan < 1% latency overhead for policy layers
  - Implement core
- Estimate: ~2h
- Scope: In scope: < 1% latency overhead for policy layers. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P10.05.b] < 1% latency overhead for policy layers
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: < 1% latency overhead for policy layers. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P10.06.a] < 10μs per event emission
- Steps:
  - Plan < 10μs per event emission
  - Implement core
- Estimate: ~2h
- Scope: In scope: < 10μs per event emission. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P10.06.b] < 10μs per event emission
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: < 10μs per event emission. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P10.07.a] Lock-free fast paths where possible
- Steps:
  - Plan Lock-free fast paths where possible
  - Implement core
- Estimate: ~2h
- Scope: In scope: Lock-free fast paths where possible. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P10.07.b] Lock-free fast paths where possible
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Lock-free fast paths where possible. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P10.08.a] Chaos engineering tests
- Steps:
  - Plan Chaos engineering tests
  - Implement core
- Estimate: ~2h
- Scope: In scope: Chaos engineering tests. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P10.08.b] Chaos engineering tests
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Chaos engineering tests. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P10.09.a] Soak tests (run for days)
- Steps:
  - Plan Soak tests (run for days)
  - Implement core
- Estimate: ~2h
- Scope: In scope: Soak tests (run for days). Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P10.09.b] Soak tests (run for days)
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Soak tests (run for days). Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P10.10.a] Load tests (thousands of RPS)
- Steps:
  - Plan Load tests (thousands of RPS)
  - Implement core
- Estimate: ~2h
- Scope: In scope: Load tests (thousands of RPS). Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P10.10.b] Load tests (thousands of RPS)
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Load tests (thousands of RPS). Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P10.11.a] Failure injection tests
- Steps:
  - Plan Failure injection tests
  - Implement core
- Estimate: ~2h
- Scope: In scope: Failure injection tests. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P10.11.b] Failure injection tests
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Failure injection tests. Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P10.12.a] Add detailed tracing spans to all layers
- Steps:
  - Plan Add detailed tracing spans to all layers
  - Implement core
- Estimate: ~2h
- Scope: In scope: Add detailed tracing spans to all layers. Out of scope: unrelated transports/services.
- Complexity: ~50 LoC

### [P10.12.b] Add detailed tracing spans to all layers
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Add detailed tracing spans to all layers. Out of scope: unrelated transports/services.
- Complexity: ~50 LoC

### [P10.13.a] Metrics integration (Prometheus)
- Steps:
  - Plan Metrics integration (Prometheus)
  - Implement core
- Estimate: ~2h
- Scope: In scope: Metrics integration (Prometheus). Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P10.13.b] Metrics integration (Prometheus)
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Metrics integration (Prometheus). Out of scope: unrelated transports/services.
- Complexity: ~40 LoC

### [P10.14.a] Add flame graph generation support
- Steps:
  - Plan Add flame graph generation support
  - Implement core
- Estimate: ~2h
- Scope: In scope: Add flame graph generation support. Out of scope: unrelated transports/services.
- Complexity: ~50 LoC

### [P10.14.b] Add flame graph generation support
- Steps:
  - Add tests
  - Document & wire up
- Estimate: ~1-2h
- Scope: In scope: Add flame graph generation support. Out of scope: unrelated transports/services.
- Complexity: ~50 LoC

## Expanded Checklist
- [ ] [P0.01] timeout.rs -> TimeoutLayer + TimeoutService
- [ ] [P0.02] retry.rs -> RetryLayer + RetryService
- [ ] [P0.03] bulkhead.rs -> BulkheadLayer + BulkheadService
- [ ] [P0.04] circuit_breaker.rs -> CircuitBreakerLayer + CircuitBreakerService
- [ ] [P0.05] Backoff/Jitter integration with retry layer
- [ ] [P0.06] Policy wrapper
- [ ] [P0.07] `+` operator (CombinedLayer for sequential composition)
- [ ] [P0.08] `|` operator (FallbackLayer for failover)
- [ ] [P0.09] Doc comments and examples for algebra API
- [ ] [P0.10] Operator precedence documentation
- [ ] [P0.11] Delete src/stack.rs
- [ ] [P0.12] Remove ResilienceStack exports from lib/prelude
- [ ] [P0.13] Delete examples/full_stack.rs
- [ ] [P0.14] Remove legacy references from tests
- [ ] [P0.15] Update lib.rs with new quick start (Policy + tower ServiceBuilder)
- [ ] [P0.16] Update prelude.rs with algebra re-exports
- [ ] [P0.17] Create README.md with:
- [ ] [P0.18] Update examples/:
- [ ] [P0.19] Add doc tests for algebra operators
- [ ] [P0.20] Adapt integration tests to Layer/Service architecture
- [ ] [P0.21] Add test coverage for:
- [ ] [P0.22] Ensure clippy passes
- [ ] [P0.23] Ensure all doc tests pass
- [ ] [P0.24] Add CI workflow if missing
- [ ] [P1.01] Define `PolicyEvent` enum:
- [ ] [P1.02] Add event emission to all policy layers:
- [ ] [P1.03] Define `TelemetrySink` as a `tower::Service<PolicyEvent, Response=(), Error=E>`
- [ ] [P1.04] Implement basic sinks:
- [ ] [P1.05] Wire policies to accept telemetry sink via `.with_sink()` method
- [ ] [P1.06] Implement `MulticastSink` for sending to multiple sinks (multicast to both)
- [ ] [P1.07] Implement `FallbackSink` for fallback on failure
- [ ] [P1.08] Add `ComposedSinkError` type for composition errors
- [ ] [P1.09] Document sink composition patterns
- [ ] [P1.10] Thread sink through policy constructors/builders via `.with_sink()` method
- [ ] [P1.11] Add examples showing telemetry integration:
- [ ] [P1.12] Benchmark overhead of event emission (deferred to Phase 10)
- [ ] [P2.01] Design `Adaptive<T>` wrapper:
- [ ] [P2.02] Integrate Adaptive into policy configs:
- [ ] [P2.03] Define `CommandContext` struct:
- [ ] [P2.04] Define `CommandHandler` trait as `tower::Service<CommandContext>`
- [ ] [P2.05] Implement `ControlPlaneRouter`:
- [ ] [P2.06] `SetParameterHandler` (update Adaptive values)
- [ ] [P2.07] `GetParameterHandler` (read current config)
- [ ] [P2.08] `GetStateHandler` (query policy state)
- [ ] [P2.09] `ResetCircuitBreakerHandler`
- [ ] [P2.10] `ListPoliciesHandler`
- [ ] [P2.11] Start with `ninelives-control` crate
- [ ] [P2.12] Implement local/in-process transport (channels)
- [ ] [P2.13] Design transport abstraction for future HTTP/gRPC/etc.
- [ ] [P2.14] Define `AuthorizationLayer` (checks Identity in CommandContext)
- [ ] [P2.15] Define `AuditLayer` (logs all commands)
- [ ] [P2.16] Wrap ControlPlaneRouter in Policy(AuthZ) + Policy(Audit)
- [ ] [P3.01] Design `SystemState` struct:
- [ ] [P3.02] Implement efficient storage (ring buffers, sketches)
- [ ] [P3.03] Create `ninelives-observer` crate
- [ ] [P3.04] Implement `Observer` as a background task
- [ ] [P3.05] Subscribe to StreamingSink
- [ ] [P3.06] Ingest PolicyEvents and update SystemState
- [ ] [P3.07] Expose query interface:
- [ ] [P3.08] Wire Observer to telemetry message bus
- [ ] [P3.09] Add control plane commands to query Observer state
- [ ] [P3.10] Add examples showing Observer usage
- [ ] [P4.01] Design `ForkJoinLayer` and `ForkJoinService`
- [ ] [P4.02] Spawn both services concurrently (futures::select for racing)
- [ ] [P4.03] Return first `Ok` result
- [ ] [P4.04] Cancel remaining futures on first success
- [ ] [P4.05] Handle case where both fail (return error)
- [ ] [P4.06] Implement `BitAnd` trait for `Policy<L>`
- [ ] [P4.07] Returns `Policy<ForkJoinLayer<A, B>>`
- [ ] [P4.08] Test race conditions (doc tests cover both sides)
- [ ] [P4.09] Test both-fail scenarios (implemented in service logic)
- [ ] [P4.10] Test cancellation behavior (futures::select handles drop)
- [ ] [P4.11] Benchmark overhead vs sequential
- [ ] [P4.12] Add examples: IPv4/IPv6, cache strategies
- [ ] [P4.13] Document operator precedence: `&` > `+` > `|`
- [ ] [P4.14] Add to algebra guide (README, lib.rs, examples)
- [ ] [P5.01] Create `ninelives-sentinel` crate
- [ ] [P5.02] Integrate Rhai scripting engine
- [ ] [P5.03] Define script API:
- [ ] [P5.04] Implement meta-policy evaluation loop:
- [ ] [P5.05] `ReloadMetaPolicyHandler` command
- [ ] [P5.06] Watch script file for changes (optional)
- [ ] [P5.07] Validate script before activating
- [ ] [P5.08] Auto-adjust retry backoff based on error rate
- [ ] [P5.09] Open circuit breaker on sustained failures
- [ ] [P5.10] Increase bulkhead capacity under load
- [ ] [P5.11] Alert on anomalies
- [ ] [P5.12] Implement `Sentinel` as top-level coordinator:
- [ ] [P5.13] Add graceful shutdown
- [ ] [P6.01] Extend Adaptive<T> to support shadow values:
- [ ] [P6.02] Add shadow mode to policy layers:
- [ ] [P6.03] Define `ShadowEvent` type (includes primary + shadow results)
- [ ] [P6.04] Add shadow event emission to policies
- [ ] [P6.05] Observer ingests shadow events separately
- [ ] [P6.06] Sentinel observes shadow stability over time window
- [ ] [P6.07] Issues `PromoteShadowHandler` command if stable
- [ ] [P6.08] Policies atomically swap shadow -> primary
- [ ] [P6.09] Ensure shadow evaluation doesn't affect primary path latency
- [ ] [P6.10] Add circuit breaker to kill shadow eval if too expensive
- [ ] [P6.11] Document safety guarantees
- [ ] [P7.01] Create workspace Cargo.toml
- [ ] [P7.02] Split crates:
- [ ] [P7.03] Update dependencies and re-exports
- [ ] [P7.04] Ensure backward compatibility
- [ ] [P7.05] Create adapter template/guide
- [ ] [P7.06] Implement priority adapters:
- [ ] [P7.07] Document adapter development
- [ ] [P8.01] Design transport-agnostic command serialization
- [ ] [P8.02] Support JSON and/or MessagePack
- [ ] [P8.03] Create `ninelives-rest` crate
- [ ] [P8.04] Expose ControlPlaneRouter over HTTP endpoints
- [ ] [P8.05] Add authentication middleware
- [ ] [P8.06] `ninelives-graphql` (GraphQL API)
- [ ] [P8.07] `ninelives-mcp` (Model Context Protocol)
- [ ] [P8.08] `ninelives-grpc` (gRPC service)
- [ ] [P9.01] Autonomous Canary Releases
- [ ] [P9.02] Progressive Ratchet-Up
- [ ] [P9.03] Safety Valves (auto-scaling policies)
- [ ] [P9.04] Blue/Green Deployments
- [ ] [P9.05] Multi-Region Failover
- [ ] [P9.06] Build example apps in `examples/recipes/`
- [ ] [P9.07] Include Sentinel scripts for each pattern
- [ ] [P9.08] Add integration tests
- [ ] [P10.01] Criterion benchmarks for each layer
- [ ] [P10.02] Compare overhead vs raw service calls
- [ ] [P10.03] Profile hot paths (event emission, state checks)
- [ ] [P10.04] Optimize lock contention
- [ ] [P10.05] < 1% latency overhead for policy layers
- [ ] [P10.06] < 10μs per event emission
- [ ] [P10.07] Lock-free fast paths where possible
- [ ] [P10.08] Chaos engineering tests
- [ ] [P10.09] Soak tests (run for days)
- [ ] [P10.10] Load tests (thousands of RPS)
- [ ] [P10.11] Failure injection tests
- [ ] [P10.12] Add detailed tracing spans to all layers
- [ ] [P10.13] Metrics integration (Prometheus)
- [ ] [P10.14] Add flame graph generation support
- [ ] [P0.01.a] timeout.rs -> TimeoutLayer + TimeoutService
- [ ] [P0.01.b] timeout.rs -> TimeoutLayer + TimeoutService
- [ ] [P0.02.a] retry.rs -> RetryLayer + RetryService
- [ ] [P0.02.b] retry.rs -> RetryLayer + RetryService
- [ ] [P0.03.a] bulkhead.rs -> BulkheadLayer + BulkheadService
- [ ] [P0.03.b] bulkhead.rs -> BulkheadLayer + BulkheadService
- [ ] [P0.04.a] circuit_breaker.rs -> CircuitBreakerLayer + CircuitBreakerService
- [ ] [P0.04.b] circuit_breaker.rs -> CircuitBreakerLayer + CircuitBreakerService
- [ ] [P0.05.a] Backoff/Jitter integration with retry layer
- [ ] [P0.05.b] Backoff/Jitter integration with retry layer
- [ ] [P0.06.a] Policy wrapper
- [ ] [P0.06.b] Policy wrapper
- [ ] [P0.07.a] `+` operator (CombinedLayer for sequential composition)
- [ ] [P0.07.b] `+` operator (CombinedLayer for sequential composition)
- [ ] [P0.08.a] `|` operator (FallbackLayer for failover)
- [ ] [P0.08.b] `|` operator (FallbackLayer for failover)
- [ ] [P0.11.a] Delete src/stack.rs
- [ ] [P0.11.b] Delete src/stack.rs
- [ ] [P0.12.a] Remove ResilienceStack exports from lib/prelude
- [ ] [P0.12.b] Remove ResilienceStack exports from lib/prelude
- [ ] [P0.13.a] Delete examples/full_stack.rs
- [ ] [P0.13.b] Delete examples/full_stack.rs
- [ ] [P0.14.a] Remove legacy references from tests
- [ ] [P0.14.b] Remove legacy references from tests
- [ ] [P0.15.a] Update lib.rs with new quick start (Policy + tower ServiceBuilder)
- [ ] [P0.15.b] Update lib.rs with new quick start (Policy + tower ServiceBuilder)
- [ ] [P0.16.a] Update prelude.rs with algebra re-exports
- [ ] [P0.16.b] Update prelude.rs with algebra re-exports
- [ ] [P0.17.a] Create README.md with:
- [ ] [P0.17.b] Create README.md with:
- [ ] [P0.18.a] Update examples/:
- [ ] [P0.18.b] Update examples/:
- [ ] [P0.20.a] Adapt integration tests to Layer/Service architecture
- [ ] [P0.20.b] Adapt integration tests to Layer/Service architecture
- [ ] [P0.21.a] Add test coverage for:
- [ ] [P0.21.b] Add test coverage for:
- [ ] [P0.22.a] Ensure clippy passes
- [ ] [P0.22.b] Ensure clippy passes
- [ ] [P0.24.a] Add CI workflow if missing
- [ ] [P0.24.b] Add CI workflow if missing
- [ ] [P1.01.a] Define `PolicyEvent` enum:
- [ ] [P1.01.b] Define `PolicyEvent` enum:
- [ ] [P1.02.a] Add event emission to all policy layers:
- [ ] [P1.02.b] Add event emission to all policy layers:
- [ ] [P1.03.a] Define `TelemetrySink` as a `tower::Service<PolicyEvent, Response=(), Error=E>`
- [ ] [P1.03.b] Define `TelemetrySink` as a `tower::Service<PolicyEvent, Response=(), Error=E>`
- [ ] [P1.04.a] Implement basic sinks:
- [ ] [P1.04.b] Implement basic sinks:
- [ ] [P1.05.a] Wire policies to accept telemetry sink via `.with_sink()` method
- [ ] [P1.05.b] Wire policies to accept telemetry sink via `.with_sink()` method
- [ ] [P1.06.a] Implement `MulticastSink` for sending to multiple sinks (multicast to both)
- [ ] [P1.06.b] Implement `MulticastSink` for sending to multiple sinks (multicast to both)
- [ ] [P1.07.a] Implement `FallbackSink` for fallback on failure
- [ ] [P1.07.b] Implement `FallbackSink` for fallback on failure
- [ ] [P1.08.a] Add `ComposedSinkError` type for composition errors
- [ ] [P1.08.b] Add `ComposedSinkError` type for composition errors
- [ ] [P1.10.a] Thread sink through policy constructors/builders via `.with_sink()` method
- [ ] [P1.10.b] Thread sink through policy constructors/builders via `.with_sink()` method
- [ ] [P1.11.a] Add examples showing telemetry integration:
- [ ] [P1.11.b] Add examples showing telemetry integration:
- [ ] [P1.12.a] Benchmark overhead of event emission (deferred to Phase 10)
- [ ] [P1.12.b] Benchmark overhead of event emission (deferred to Phase 10)
- [ ] [P2.01.a] Design `Adaptive<T>` wrapper:
- [ ] [P2.01.b] Design `Adaptive<T>` wrapper:
- [ ] [P2.02.a] Integrate Adaptive into policy configs:
- [ ] [P2.02.b] Integrate Adaptive into policy configs:
- [ ] [P2.03.a] Define `CommandContext` struct:
- [ ] [P2.03.b] Define `CommandContext` struct:
- [ ] [P2.04.a] Define `CommandHandler` trait as `tower::Service<CommandContext>`
- [ ] [P2.04.b] Define `CommandHandler` trait as `tower::Service<CommandContext>`
- [ ] [P2.05.a] Implement `ControlPlaneRouter`:
- [ ] [P2.05.b] Implement `ControlPlaneRouter`:
- [ ] [P2.06.a] `SetParameterHandler` (update Adaptive values)
- [ ] [P2.06.b] `SetParameterHandler` (update Adaptive values)
- [ ] [P2.07.a] `GetParameterHandler` (read current config)
- [ ] [P2.07.b] `GetParameterHandler` (read current config)
- [ ] [P2.08.a] `GetStateHandler` (query policy state)
- [ ] [P2.08.b] `GetStateHandler` (query policy state)
- [ ] [P2.09.a] `ResetCircuitBreakerHandler`
- [ ] [P2.09.b] `ResetCircuitBreakerHandler`
- [ ] [P2.10.a] `ListPoliciesHandler`
- [ ] [P2.10.b] `ListPoliciesHandler`
- [ ] [P2.11.a] Start with `ninelives-control` crate
- [ ] [P2.11.b] Start with `ninelives-control` crate
- [ ] [P2.12.a] Implement local/in-process transport (channels)
- [ ] [P2.12.b] Implement local/in-process transport (channels)
- [ ] [P2.13.a] Design transport abstraction for future HTTP/gRPC/etc.
- [ ] [P2.13.b] Design transport abstraction for future HTTP/gRPC/etc.
- [ ] [P2.14.a] Define `AuthorizationLayer` (checks Identity in CommandContext)
- [ ] [P2.14.b] Define `AuthorizationLayer` (checks Identity in CommandContext)
- [ ] [P2.15.a] Define `AuditLayer` (logs all commands)
- [ ] [P2.15.b] Define `AuditLayer` (logs all commands)
- [ ] [P2.16.a] Wrap ControlPlaneRouter in Policy(AuthZ) + Policy(Audit)
- [ ] [P2.16.b] Wrap ControlPlaneRouter in Policy(AuthZ) + Policy(Audit)
- [ ] [P3.01.a] Design `SystemState` struct:
- [ ] [P3.01.b] Design `SystemState` struct:
- [ ] [P3.02.a] Implement efficient storage (ring buffers, sketches)
- [ ] [P3.02.b] Implement efficient storage (ring buffers, sketches)
- [ ] [P3.03.a] Create `ninelives-observer` crate
- [ ] [P3.03.b] Create `ninelives-observer` crate
- [ ] [P3.04.a] Implement `Observer` as a background task
- [ ] [P3.04.b] Implement `Observer` as a background task
- [ ] [P3.05.a] Subscribe to StreamingSink
- [ ] [P3.05.b] Subscribe to StreamingSink
- [ ] [P3.06.a] Ingest PolicyEvents and update SystemState
- [ ] [P3.06.b] Ingest PolicyEvents and update SystemState
- [ ] [P3.07.a] Expose query interface:
- [ ] [P3.07.b] Expose query interface:
- [ ] [P3.08.a] Wire Observer to telemetry message bus
- [ ] [P3.08.b] Wire Observer to telemetry message bus
- [ ] [P3.09.a] Add control plane commands to query Observer state
- [ ] [P3.09.b] Add control plane commands to query Observer state
- [ ] [P3.10.a] Add examples showing Observer usage
- [ ] [P3.10.b] Add examples showing Observer usage
- [ ] [P4.01.a] Design `ForkJoinLayer` and `ForkJoinService`
- [ ] [P4.01.b] Design `ForkJoinLayer` and `ForkJoinService`
- [ ] [P4.02.a] Spawn both services concurrently (futures::select for racing)
- [ ] [P4.02.b] Spawn both services concurrently (futures::select for racing)
- [ ] [P4.03.a] Return first `Ok` result
- [ ] [P4.03.b] Return first `Ok` result
- [ ] [P4.04.a] Cancel remaining futures on first success
- [ ] [P4.04.b] Cancel remaining futures on first success
- [ ] [P4.05.a] Handle case where both fail (return error)
- [ ] [P4.05.b] Handle case where both fail (return error)
- [ ] [P4.06.a] Implement `BitAnd` trait for `Policy<L>`
- [ ] [P4.06.b] Implement `BitAnd` trait for `Policy<L>`
- [ ] [P4.07.a] Returns `Policy<ForkJoinLayer<A, B>>`
- [ ] [P4.07.b] Returns `Policy<ForkJoinLayer<A, B>>`
- [ ] [P4.09.a] Test both-fail scenarios (implemented in service logic)
- [ ] [P4.09.b] Test both-fail scenarios (implemented in service logic)
- [ ] [P4.10.a] Test cancellation behavior (futures::select handles drop)
- [ ] [P4.10.b] Test cancellation behavior (futures::select handles drop)
- [ ] [P4.11.a] Benchmark overhead vs sequential
- [ ] [P4.11.b] Benchmark overhead vs sequential
- [ ] [P4.12.a] Add examples: IPv4/IPv6, cache strategies
- [ ] [P4.12.b] Add examples: IPv4/IPv6, cache strategies
- [ ] [P4.14.a] Add to algebra guide (README, lib.rs, examples)
- [ ] [P4.14.b] Add to algebra guide (README, lib.rs, examples)
- [ ] [P5.01.a] Create `ninelives-sentinel` crate
- [ ] [P5.01.b] Create `ninelives-sentinel` crate
- [ ] [P5.02.a] Integrate Rhai scripting engine
- [ ] [P5.02.b] Integrate Rhai scripting engine
- [ ] [P5.03.a] Define script API:
- [ ] [P5.03.b] Define script API:
- [ ] [P5.04.a] Implement meta-policy evaluation loop:
- [ ] [P5.04.b] Implement meta-policy evaluation loop:
- [ ] [P5.05.a] `ReloadMetaPolicyHandler` command
- [ ] [P5.05.b] `ReloadMetaPolicyHandler` command
- [ ] [P5.06.a] Watch script file for changes (optional)
- [ ] [P5.06.b] Watch script file for changes (optional)
- [ ] [P5.07.a] Validate script before activating
- [ ] [P5.07.b] Validate script before activating
- [ ] [P5.08.a] Auto-adjust retry backoff based on error rate
- [ ] [P5.08.b] Auto-adjust retry backoff based on error rate
- [ ] [P5.09.a] Open circuit breaker on sustained failures
- [ ] [P5.09.b] Open circuit breaker on sustained failures
- [ ] [P5.10.a] Increase bulkhead capacity under load
- [ ] [P5.10.b] Increase bulkhead capacity under load
- [ ] [P5.11.a] Alert on anomalies
- [ ] [P5.11.b] Alert on anomalies
- [ ] [P5.12.a] Implement `Sentinel` as top-level coordinator:
- [ ] [P5.12.b] Implement `Sentinel` as top-level coordinator:
- [ ] [P5.13.a] Add graceful shutdown
- [ ] [P5.13.b] Add graceful shutdown
- [ ] [P6.01.a] Extend Adaptive<T> to support shadow values:
- [ ] [P6.01.b] Extend Adaptive<T> to support shadow values:
- [ ] [P6.02.a] Add shadow mode to policy layers:
- [ ] [P6.02.b] Add shadow mode to policy layers:
- [ ] [P6.03.a] Define `ShadowEvent` type (includes primary + shadow results)
- [ ] [P6.03.b] Define `ShadowEvent` type (includes primary + shadow results)
- [ ] [P6.04.a] Add shadow event emission to policies
- [ ] [P6.04.b] Add shadow event emission to policies
- [ ] [P6.05.a] Observer ingests shadow events separately
- [ ] [P6.05.b] Observer ingests shadow events separately
- [ ] [P6.06.a] Sentinel observes shadow stability over time window
- [ ] [P6.06.b] Sentinel observes shadow stability over time window
- [ ] [P6.07.a] Issues `PromoteShadowHandler` command if stable
- [ ] [P6.07.b] Issues `PromoteShadowHandler` command if stable
- [ ] [P6.08.a] Policies atomically swap shadow -> primary
- [ ] [P6.08.b] Policies atomically swap shadow -> primary
- [ ] [P6.09.a] Ensure shadow evaluation doesn't affect primary path latency
- [ ] [P6.09.b] Ensure shadow evaluation doesn't affect primary path latency
- [ ] [P6.10.a] Add circuit breaker to kill shadow eval if too expensive
- [ ] [P6.10.b] Add circuit breaker to kill shadow eval if too expensive
- [ ] [P7.01.a] Create workspace Cargo.toml
- [ ] [P7.01.b] Create workspace Cargo.toml
- [ ] [P7.02.a] Split crates:
- [ ] [P7.02.b] Split crates:
- [ ] [P7.03.a] Update dependencies and re-exports
- [ ] [P7.03.b] Update dependencies and re-exports
- [ ] [P7.04.a] Ensure backward compatibility
- [ ] [P7.04.b] Ensure backward compatibility
- [ ] [P7.05.a] Create adapter template/guide
- [ ] [P7.05.b] Create adapter template/guide
- [ ] [P7.06.a] Implement priority adapters:
- [ ] [P7.06.b] Implement priority adapters:
- [ ] [P8.01.a] Design transport-agnostic command serialization
- [ ] [P8.01.b] Design transport-agnostic command serialization
- [ ] [P8.02.a] Support JSON and/or MessagePack
- [ ] [P8.02.b] Support JSON and/or MessagePack
- [ ] [P8.03.a] Create `ninelives-rest` crate
- [ ] [P8.03.b] Create `ninelives-rest` crate
- [ ] [P8.04.a] Expose ControlPlaneRouter over HTTP endpoints
- [ ] [P8.04.b] Expose ControlPlaneRouter over HTTP endpoints
- [ ] [P8.05.a] Add authentication middleware
- [ ] [P8.05.b] Add authentication middleware
- [ ] [P8.06.a] `ninelives-graphql` (GraphQL API)
- [ ] [P8.06.b] `ninelives-graphql` (GraphQL API)
- [ ] [P8.07.a] `ninelives-mcp` (Model Context Protocol)
- [ ] [P8.07.b] `ninelives-mcp` (Model Context Protocol)
- [ ] [P8.08.a] `ninelives-grpc` (gRPC service)
- [ ] [P8.08.b] `ninelives-grpc` (gRPC service)
- [ ] [P9.01.a] Autonomous Canary Releases
- [ ] [P9.01.b] Autonomous Canary Releases
- [ ] [P9.02.a] Progressive Ratchet-Up
- [ ] [P9.02.b] Progressive Ratchet-Up
- [ ] [P9.03.a] Safety Valves (auto-scaling policies)
- [ ] [P9.03.b] Safety Valves (auto-scaling policies)
- [ ] [P9.04.a] Blue/Green Deployments
- [ ] [P9.04.b] Blue/Green Deployments
- [ ] [P9.05.a] Multi-Region Failover
- [ ] [P9.05.b] Multi-Region Failover
- [ ] [P9.06.a] Build example apps in `examples/recipes/`
- [ ] [P9.06.b] Build example apps in `examples/recipes/`
- [ ] [P9.07.a] Include Sentinel scripts for each pattern
- [ ] [P9.07.b] Include Sentinel scripts for each pattern
- [ ] [P9.08.a] Add integration tests
- [ ] [P9.08.b] Add integration tests
- [ ] [P10.01.a] Criterion benchmarks for each layer
- [ ] [P10.01.b] Criterion benchmarks for each layer
- [ ] [P10.02.a] Compare overhead vs raw service calls
- [ ] [P10.02.b] Compare overhead vs raw service calls
- [ ] [P10.03.a] Profile hot paths (event emission, state checks)
- [ ] [P10.03.b] Profile hot paths (event emission, state checks)
- [ ] [P10.04.a] Optimize lock contention
- [ ] [P10.04.b] Optimize lock contention
- [ ] [P10.05.a] < 1% latency overhead for policy layers
- [ ] [P10.05.b] < 1% latency overhead for policy layers
- [ ] [P10.06.a] < 10μs per event emission
- [ ] [P10.06.b] < 10μs per event emission
- [ ] [P10.07.a] Lock-free fast paths where possible
- [ ] [P10.07.b] Lock-free fast paths where possible
- [ ] [P10.08.a] Chaos engineering tests
- [ ] [P10.08.b] Chaos engineering tests
- [ ] [P10.09.a] Soak tests (run for days)
- [ ] [P10.09.b] Soak tests (run for days)
- [ ] [P10.10.a] Load tests (thousands of RPS)
- [ ] [P10.10.b] Load tests (thousands of RPS)
- [ ] [P10.11.a] Failure injection tests
- [ ] [P10.11.b] Failure injection tests
- [ ] [P10.12.a] Add detailed tracing spans to all layers
- [ ] [P10.12.b] Add detailed tracing spans to all layers
- [ ] [P10.13.a] Metrics integration (Prometheus)
- [ ] [P10.13.b] Metrics integration (Prometheus)
- [ ] [P10.14.a] Add flame graph generation support
- [ ] [P10.14.b] Add flame graph generation support
