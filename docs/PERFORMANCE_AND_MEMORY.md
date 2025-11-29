# Performance & Memory Recommendations

Based on a deep dive into the `ninelives` architecture, this document outlines specific recommendations to optimize performance (latency/throughput) and memory usage. These should be prioritized during **Phase 10 (Production Hardening)** but kept in mind during all development.

## 1. CPU & Latency

### 1.1. Eliminate `BoxFuture` in Core Services

**Status**: Critical optimization.
**Issue**: Currently, core services like `CircuitBreakerService`, `FallbackService`, and `ForkJoinService` use `BoxFuture`:

```rust
type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;
```

This forces a heap allocation for *every single request*. In a high-throughput system (100k+ RPS), this allocator pressure is significant.

**Recommendation**:
Rewrite these services to define custom `Future` structs using `pin-project` or `pin-project-lite`. This allows the future state to be stack-allocated (or embedded in the parent future), enabling "zero-allocation" request paths.

### 1.2. Reduce `Arc` Cloning on Hot Paths

**Status**: Important optimization.
**Issue**: In `CircuitBreakerService::call`, we clone 4 `Arc`s per request:

```rust
let state = self.state.clone();
let config = self.config.clone();
let clock = self.clock.clone();
let sink = self.sink.clone();
```

This results in 4 atomic increments and 4 atomic decrements per request.

**Recommendation**:
Group these shared components into a single `Arc<Context>` struct.

```rust
struct CircuitBreakerContext<S, Sink> {
    inner: S,
    state: CircuitBreakerState,
    config: CircuitBreakerConfig,
    clock: Box<dyn Clock>, // or generic
    sink: Sink,
}
```

If `S` (the inner service) is `Clone`, this might be tricky, but for shared state components (`state`, `config`, `clock`), bundling them reduces atomic contention.

### 1.3. Telemetry Overhead

**Status**: Usage pattern recommendation.
**Issue**: `emit_best_effort` awaits the sink. If the sink does any work (locks a mutex, writes to I/O), it adds latency directly to the user's request *after* the inner service has finished.

**Recommendation**:

* **Always** wrap I/O-bound sinks (Log, Prometheus, OTLP) in `NonBlockingSink`.
* Ensure `NonBlockingSink` uses a bounded channel to provide backpressure/dropping rather than unbounded memory growth.
* Consider a "fire-and-forget" path where `emit` doesn't even await the channel send if the channel is full (drop immediately).

## 2. Memory

### 2.1. `PolicyEvent` Size

**Status**: Monitor.
**Issue**: `PolicyEvent` is an enum. Its size is equal to the largest variant + discriminant. If one variant grows large, all events pay the cost.

**Recommendation**:

* Keep event variants small.
* Box large or rare variants.
* Run `cargo size-check` (or similar) on the enum to be aware of its footprint.

### 2.2. Adaptive Configuration

**Status**: Good.
**Observation**: `Adaptive<T>` uses `arc-swap`, which is excellent for read-heavy, write-rare workloads. Reads are wait-free.
**Recommendation**: Continue using this pattern. Avoid `RwLock` for config on the hot path.

## 3. Architecture & Design

### 3.1. Zero-Copy Telemetry (Advanced)

**Status**: Future enhancement (Phase 10+).
**Idea**: Instead of creating `PolicyEvent` enums (which might copy data), allow writing directly to a pre-allocated ring buffer (LMAX Disruptor style).
**Benefit**: Removes the allocator from the telemetry path entirely.

### 3.2. `no_std` Compatibility

**Status**: Future enhancement.
**Idea**: The core logic (algebra, state machines) doesn't require `std`.
**Recommendation**: Isolate `ninelives-core` logic behind `#[no_std]` with `alloc` feature. This forces strict discipline regarding allocations and makes the library usable in embedded contexts (firmware resilience).

## 4. Summary of Action Items for Phase 10

1. [ ] **Refactor to Unboxed Futures**: Convert `Retry`, `CircuitBreaker`, `Fallback`, `ForkJoin` to use `pin-project`.
2. [ ] **Context Structs**: Bundle `Arc` fields to reduce atomic ref-counting.
3. [ ] **Benchmark Baseline**: Establish current overhead (ns/op) before applying these optimizations.
