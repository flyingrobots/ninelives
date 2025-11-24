# Nine Lives v2: Roadmap to the Fractal Future

**Mission:** Build the autonomous, algebraic resilience framework for distributed systems.

---

## Phase 0: Foundation Cleanup (PRIORITY: IMMEDIATE)

**Goal:** Complete the tower-native refactor and establish a stable v1.0 API surface.

### Core Tower Migration
- [x] timeout.rs -> TimeoutLayer + TimeoutService
- [x] retry.rs -> RetryLayer + RetryService
- [x] bulkhead.rs -> BulkheadLayer + BulkheadService
- [x] circuit_breaker.rs -> CircuitBreakerLayer + CircuitBreakerService
- [x] Backoff/Jitter integration with retry layer

### Algebra DSL - v1 (Sequential + Fallback)
- [x] Policy wrapper
- [x] `+` operator (CombinedLayer for sequential composition)
- [x] `|` operator (FallbackLayer for failover)
- [x] Doc comments and examples for algebra API
- [x] Operator precedence documentation

### Cleanup Legacy Code
- [x] Delete src/stack.rs
- [x] Remove ResilienceStack exports from lib/prelude
- [x] Delete examples/full_stack.rs
- [x] Remove legacy references from tests

### Documentation & Examples
- [x] Update lib.rs with new quick start (Policy + tower ServiceBuilder)
- [x] Update prelude.rs with algebra re-exports
- [ ] Create README.md with:
  - Algebraic composition examples
  - Tower integration guide
  - Quick start with `Policy(A) + Policy(B) | Policy(C)`
- [ ] Update examples/:
  - [ ] retry_only.rs (tower-native)
  - [ ] decorrelated_jitter.rs (tower-native)
  - [ ] Create algebra_composition.rs example
- [ ] Add doc tests for algebra operators

### Testing & CI
- [ ] Adapt integration tests to Layer/Service architecture
- [ ] Add test coverage for:
  - [ ] Combined composition (`A + B`)
  - [ ] Fallback composition (`A | B`)
  - [ ] Nested composition (`A | B + C`)
- [ ] Ensure clippy passes
- [ ] Ensure all doc tests pass
- [ ] Add CI workflow if missing

**Milestone:** Publish `ninelives` v1.0.0 - The tower-native algebraic resilience library

---

## Phase 1: The Telemetry & Message Plane (PRIORITY: HIGH)

**Goal:** Build the observability foundation that enables autonomous operation.

### Event System
- [ ] Define `PolicyEvent` enum:
  - [ ] RetryAttempt, RetryExhausted
  - [ ] CircuitOpened, CircuitClosed, CircuitHalfOpen
  - [ ] BulkheadRejected, BulkheadAcquired
  - [ ] TimeoutOccurred
  - [ ] RequestSuccess, RequestFailure
- [ ] Add event emission to all policy layers:
  - [ ] RetryService emits on each attempt
  - [ ] CircuitBreakerService emits on state transitions
  - [ ] BulkheadService emits on acquire/reject
  - [ ] TimeoutService emits on timeout

### TelemetrySink Abstraction
- [ ] Define `TelemetrySink` as a `tower::Service<PolicyEvent, Response=(), Error=E>`
- [ ] Implement basic sinks:
  - [ ] `NullSink` (no-op for testing)
  - [ ] `LogSink` (logs events via `tracing`)
  - [ ] `MemorySink` (in-memory buffer for testing)
  - [ ] `StreamingSink` (tokio::sync::broadcast pub/sub bus)
- [ ] Wire policies to accept `Arc<dyn TelemetrySink>` (or generic)

### Algebraic Sink Composition
- [ ] Implement `+` for TelemetrySink (multicast to both)
- [ ] Implement `|` for TelemetrySink (fallback on failure)
- [ ] Add tests for sink composition
- [ ] Document sink algebra patterns

### Integration
- [ ] Thread sink through policy constructors/builders
- [ ] Add examples showing telemetry integration
- [ ] Benchmark overhead of event emission

**Milestone:** `ninelives` v1.1.0 - Policies emit structured telemetry

---

## Phase 2: The Dynamic Control Plane (PRIORITY: HIGH)

**Goal:** Enable runtime policy tuning and command execution.

### Adaptive Handles
- [ ] Design `Adaptive<T>` wrapper:
  - [ ] Arc<RwLock<T>> or Arc<ArcSwap<T>> for lock-free reads
  - [ ] Methods: `get()`, `set()`, `update()`
- [ ] Integrate Adaptive into policy configs:
  - [ ] RetryPolicy: max_attempts, backoff parameters
  - [ ] CircuitBreaker: failure_threshold, timeout_duration
  - [ ] Bulkhead: max_concurrency
  - [ ] Timeout: duration

### Command System
- [ ] Define `CommandContext` struct:
  - [ ] Command name
  - [ ] Arguments (JSON or typed enum)
  - [ ] Identity (for authz)
  - [ ] Response channel
- [ ] Define `CommandHandler` trait as `tower::Service<CommandContext>`
- [ ] Implement `ControlPlaneRouter`:
  - [ ] Dynamic handler registration
  - [ ] Command dispatch by name
  - [ ] Error handling and response routing

### Built-in Command Handlers
- [ ] `SetParameterHandler` (update Adaptive values)
- [ ] `GetParameterHandler` (read current config)
- [ ] `GetStateHandler` (query policy state)
- [ ] `ResetCircuitBreakerHandler`
- [ ] `ListPoliciesHandler`

### Control Plane Transports
- [ ] Start with `ninelives-control` crate
- [ ] Implement local/in-process transport (channels)
- [ ] Design transport abstraction for future HTTP/gRPC/etc.

### Security Layer
- [ ] Define `AuthorizationLayer` (checks Identity in CommandContext)
- [ ] Define `AuditLayer` (logs all commands)
- [ ] Wrap ControlPlaneRouter in Policy(AuthZ) + Policy(Audit)

**Milestone:** `ninelives-control` v0.1.0 - Runtime policy tuning via command plane

---

## Phase 3: The Observer (PRIORITY: MEDIUM)

**Goal:** Aggregate telemetry into queryable system state.

### SystemState Model
- [ ] Design `SystemState` struct:
  - [ ] Per-policy metrics (error rate, latency percentiles, state)
  - [ ] Time-windowed aggregations (1m, 5m, 15m windows)
  - [ ] Circuit breaker states
  - [ ] Bulkhead utilization
- [ ] Implement efficient storage (ring buffers, sketches)

### Observer Service
- [ ] Create `ninelives-observer` crate
- [ ] Implement `Observer` as a background task
- [ ] Subscribe to StreamingSink
- [ ] Ingest PolicyEvents and update SystemState
- [ ] Expose query interface:
  - [ ] `get_policy_state(policy_id)`
  - [ ] `get_error_rate(policy_id, window)`
  - [ ] `get_circuit_state(policy_id)`

### Integration
- [ ] Wire Observer to telemetry message bus
- [ ] Add control plane commands to query Observer state
- [ ] Add examples showing Observer usage

**Milestone:** `ninelives-observer` v0.1.0 - Queryable system state from telemetry

---

## Phase 4: Algebra Completion - Fork-Join (`&`) (PRIORITY: MEDIUM)

**Goal:** Implement the "happy eyeballs" parallel composition operator.

### ForkJoinLayer Implementation
- [ ] Design `ForkJoinLayer` and `ForkJoinService`
- [ ] Spawn both services concurrently (tokio::spawn or FuturesUnordered)
- [ ] Return first `Ok` result
- [ ] Cancel remaining futures on first success
- [ ] Handle case where both fail (return combined error)

### Operator Overloading
- [ ] Implement `BitAnd` trait for `Policy<L>`
- [ ] Returns `Policy<ForkJoinLayer<A, B>>`

### Testing
- [ ] Test race conditions (A wins, B wins)
- [ ] Test both-fail scenarios
- [ ] Test cancellation behavior
- [ ] Benchmark overhead vs sequential

### Documentation
- [ ] Add examples: IPv4/IPv6, cache strategies
- [ ] Document operator precedence: `&` > `+` > `|`
- [ ] Add to algebra guide

**Milestone:** `ninelives` v1.2.0 - Complete algebraic operators (`+`, `|`, `&`)

---

## Phase 5: The Sentinel - Autonomous Control Loop (PRIORITY: MEDIUM-LOW)

**Goal:** Build the self-healing brain of the system.

### Meta-Policy Engine
- [ ] Create `ninelives-sentinel` crate
- [ ] Integrate Rhai scripting engine
- [ ] Define script API:
  - [ ] Access to SystemState queries
  - [ ] Issue control plane commands
  - [ ] Time-based triggers
- [ ] Implement meta-policy evaluation loop:
  - [ ] Load Rhai script
  - [ ] Evaluate on interval
  - [ ] Execute commands based on rules

### Hot-Reload Support
- [ ] `ReloadMetaPolicyHandler` command
- [ ] Watch script file for changes (optional)
- [ ] Validate script before activating

### Example Meta-Policies
- [ ] Auto-adjust retry backoff based on error rate
- [ ] Open circuit breaker on sustained failures
- [ ] Increase bulkhead capacity under load
- [ ] Alert on anomalies

### Sentinel Service
- [ ] Implement `Sentinel` as top-level coordinator:
  - [ ] Wires together Observer + ControlPlaneRouter + MetaPolicyEngine
  - [ ] Exposes unified `run()` method
- [ ] Add graceful shutdown

**Milestone:** `ninelives-sentinel` v0.1.0 - Autonomous policy tuning via Rhai scripts

---

## Phase 6: Shadow Policy Evaluation (PRIORITY: LOW)

**Goal:** Enable safe what-if analysis before applying policy changes.

### Shadow Configuration
- [ ] Extend Adaptive<T> to support shadow values:
  - [ ] `set_shadow()`, `get_shadow()`, `promote_shadow()`
- [ ] Add shadow mode to policy layers:
  - [ ] Evaluate request with both primary and shadow config
  - [ ] Emit separate `ShadowEvent` for shadow outcomes

### ShadowEvent Stream
- [ ] Define `ShadowEvent` type (includes primary + shadow results)
- [ ] Add shadow event emission to policies
- [ ] Observer ingests shadow events separately

### Promotion Logic
- [ ] Sentinel observes shadow stability over time window
- [ ] Issues `PromoteShadowHandler` command if stable
- [ ] Policies atomically swap shadow -> primary

### Testing & Safety
- [ ] Ensure shadow evaluation doesn't affect primary path latency
- [ ] Add circuit breaker to kill shadow eval if too expensive
- [ ] Document safety guarantees

**Milestone:** `ninelives-sentinel` v0.2.0 - Safe shadow policy testing in production

---

## Phase 7: Workspace & Modularity (PRIORITY: LOW)

**Goal:** Split into focused crates per the spec.

### Workspace Structure
- [ ] Create workspace Cargo.toml
- [ ] Split crates:
  - [ ] `ninelives-core` (Policy, algebra, traits)
  - [ ] `ninelives` (concrete layers, simple backends)
  - [ ] `ninelives-control` (command plane)
  - [ ] `ninelives-observer` (telemetry aggregation)
  - [ ] `ninelives-sentinel` (autonomous loop)
- [ ] Update dependencies and re-exports
- [ ] Ensure backward compatibility

### Adapter Ecosystem
- [ ] Create adapter template/guide
- [ ] Implement priority adapters:
  - [ ] `ninelives-redis` (state backend)
  - [ ] `ninelives-otlp` (telemetry sink)
  - [ ] `ninelives-prometheus` (metrics exporter)
- [ ] Document adapter development

**Milestone:** Nine Lives v2.0.0 - Modular workspace with adapter ecosystem

---

## Phase 8: Control Plane Transports (PRIORITY: LOW)

**Goal:** Make the control plane accessible via multiple protocols.

### Transport Abstraction
- [ ] Design transport-agnostic command serialization
- [ ] Support JSON and/or MessagePack

### HTTP/REST Transport
- [ ] Create `ninelives-rest` crate
- [ ] Expose ControlPlaneRouter over HTTP endpoints
- [ ] Add authentication middleware

### Additional Transports
- [ ] `ninelives-graphql` (GraphQL API)
- [ ] `ninelives-mcp` (Model Context Protocol)
- [ ] `ninelives-grpc` (gRPC service)

**Milestone:** Control plane accessible via HTTP/GraphQL/gRPC

---

## Phase 9: Advanced Patterns & Recipes (PRIORITY: LOW)

**Goal:** Demonstrate high-level distributed systems patterns.

### Recipe Documentation
- [ ] Autonomous Canary Releases
- [ ] Progressive Ratchet-Up
- [ ] Safety Valves (auto-scaling policies)
- [ ] Blue/Green Deployments
- [ ] Multi-Region Failover

### Reference Implementations
- [ ] Build example apps in `examples/recipes/`
- [ ] Include Sentinel scripts for each pattern
- [ ] Add integration tests

**Milestone:** Nine Lives v2.1.0 - Production-ready recipes

---

## Phase 10: Performance & Production Hardening (PRIORITY: ONGOING)

**Goal:** Optimize for zero-overhead and production reliability.

### Benchmarking
- [ ] Criterion benchmarks for each layer
- [ ] Compare overhead vs raw service calls
- [ ] Profile hot paths (event emission, state checks)
- [ ] Optimize lock contention

### Performance Targets
- [ ] < 1% latency overhead for policy layers
- [ ] < 10μs per event emission
- [ ] Lock-free fast paths where possible

### Production Testing
- [ ] Chaos engineering tests
- [ ] Soak tests (run for days)
- [ ] Load tests (thousands of RPS)
- [ ] Failure injection tests

### Observability
- [ ] Add detailed tracing spans to all layers
- [ ] Metrics integration (Prometheus)
- [ ] Add flame graph generation support

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
