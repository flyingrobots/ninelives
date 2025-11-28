# Phase 7: Modular Ecosystem

**Status:** 📋 Planned

## Executive Summary
*   **Story:** One size does not fit all. A monolithic library creates bloat and forces unnecessary dependencies. We refactor `ninelives` into a composable ecosystem of crates, allowing users to pick exactly what they need—whether it's just the core primitives, the full control plane, or specialized adapters.
*   **Outcome:** A flexible, lightweight architecture with a minimal `ninelives-core` suitable for everything from embedded devices to massive microservices, supported by a rich ecosystem of optional extensions.

## Tasks

### P7.01a Workspace Setup

| field | value |
| --- | --- |
| id | P7.01a |
| title | Workspace Setup |
| estimate | 2h |
| status | open |
| blocked_by | - |
| blocks | - |
| value | H |

#### Summary

Convert the repository into a Cargo workspace.

#### Steps
1.  Move current [`src/`](src/) to `crates/ninelives-core/src` (temporarily or strictly).
2.  Create root `Cargo.toml` with `[workspace]`.
3.  Define members: `crates/*`, `ninelives-cookbook`.

#### Definition of Done
- [ ] `cargo build` works at root.

### P7.01b Extract Primitives

| field | value |
| --- | --- |
| id | P7.01b |
| title | Extract Primitives |
| estimate | 2h |
| status | blocked |
| blocked_by | - |
| blocks | - |
| value | H |

#### Summary

Populate `ninelives-core` with fundamental types.

#### Steps
1.  Move `Policy`, `Layer` traits, `Error` types.
2.  Move `Clock`, `Jitter`, `Backoff`.
3.  Ensure no heavy deps (like `tokio` full).

#### Definition of Done
- [ ] Core primitives compile in new crate.

### P7.01c Extract Layers

| field | value |
| --- | --- |
| id | P7.01c |
| title | Extract Layers |
| estimate | 2h |
| status | blocked |
| blocked_by | - |
| blocks | - |
| value | H |

#### Summary

Move standard layers to `ninelives-core`.

#### Steps
1.  Move `RetryLayer`, `TimeoutLayer`, `BulkheadLayer`, `CircuitBreakerLayer`.
2.  Move `ForkJoinLayer`, `ShadowLayer`.
3.  Update internal imports.

#### Definition of Done
- [ ] Layers compile in `ninelives-core`.

### P7.01d Meta-Crate Setup

| field | value |
| --- | --- |
| id | P7.01d |
| title | Meta-Crate Setup |
| estimate | 2h |
| status | blocked |
| blocked_by | - |
| blocks | - |
| value | H |

#### Summary

Create the top-level `ninelives` crate.

#### Steps
1.  Create `crates/ninelives`.
2.  Add dependency on `ninelives-core`.
3.  `pub use ninelives_core::*;`.

#### Definition of Done
- [ ] `ninelives` crate re-exports core.

### P7.01e Cookbook Fixes

| field | value |
| --- | --- |
| id | P7.01e |
| title | Cookbook Fixes |
| estimate | 2h |
| status | blocked |
| blocked_by | - |
| blocks | - |
| value | H |

#### Summary

Update cookbook to use the new workspace structure.

#### Steps
1.  Update [`ninelives-cookbook/Cargo.toml`](ninelives-cookbook/Cargo.toml) to depend on `ninelives` (path).
2.  Fix any import issues.

#### Definition of Done
- [ ] Examples compile.

### P7.01f CI Updates

| field | value |
| --- | --- |
| id | P7.01f |
| title | CI Updates |
| estimate | 2h |
| status | blocked |
| blocked_by | - |
| blocks | - |
| value | H |

#### Summary

Update GitHub Actions for workspace.

#### Steps
1.  Update `ci.yml` to run `cargo test --workspace`.
2.  Check for new crate paths.

#### Definition of Done
- [ ] CI passes.

### P7.01g Refactor to Unboxed Futures

| field | value |
| --- | --- |
| id | P7.01g |
| title | Refactor to Unboxed Futures |
| estimate | 4h |
| status | blocked |
| blocked_by | - |
| blocks | - |
| value | H |

#### Summary

Replace `BoxFuture` with `pin-project` based future structs in core services (`RetryService`, `CircuitBreakerService`, `FallbackService`, `ForkJoinService`).

#### Context

Currently, every request allocates a `BoxFuture`. In high-throughput scenarios, this allocator pressure is significant. Converting to named future structs (zero-allocation state machines) is a standard optimization in the Tower ecosystem.

#### Steps
1.  Add `pin-project` dependency.
2.  For each core service:
    - Define `pub struct ResponseFuture<...>`.
    - Implement `Future` for it.
    - Change `Service::call` to return `ResponseFuture`.

#### Definition of Done
- [ ] Core services return unboxed futures.
- [ ] `cargo check` confirms types match.

#### Test Plan
- [ ] Existing tests pass (refactor should be transparent).

### P7.02a Extract Control Crate

| field | value |
| --- | --- |
| id | P7.02a |
| title | Extract Control Crate |
| estimate | 2h |
| status | blocked |
| blocked_by | - |
| blocks | - |
| value | M |

#### Summary

Create `ninelives-control`.

#### Steps
1.  Move `CommandRouter`, `Auth`, `ConfigRegistry`.
2.  Update `ninelives` meta-crate to re-export.

#### Definition of Done
- [ ] Control logic isolated.

### P7.02b Extract Observer Crate

| field | value |
| --- | --- |
| id | P7.02b |
| title | Extract Observer Crate |
| estimate | 2h |
| status | blocked |
| blocked_by | - |
| blocks | - |
| value | M |

#### Summary

Create `ninelives-observer`.

#### Steps
1.  Move `TelemetryAggregator`, `WindowedMetrics`.
2.  Update `ninelives` meta-crate.

#### Definition of Done
- [ ] Observer logic isolated.

### P7.02c Move Sentinel Crate

| field | value |
| --- | --- |
| id | P7.02c |
| title | Move Sentinel Crate |
| estimate | 2h |
| status | blocked |
| blocked_by | - |
| blocks | - |
| value | M |

#### Summary

Move `ninelives-sentinel` to `crates/`.

#### Steps
1.  If P5 created it in root, move to `crates/`.
2.  Update workspace path.

#### Definition of Done
- [ ] Sentinel in crates dir.

### P7.02d Re-export Cleanup

| field | value |
| --- | --- |
| id | P7.02d |
| title | Re-export Cleanup |
| estimate | 2h |
| status | blocked |
| blocked_by | - |
| blocks | - |
| value | M |

#### Summary

Ensure `ninelives` prelude and exports are ergonomic.

#### Steps
1.  Check `ninelives::prelude`.
2.  Ensure feature flags allow disabling control/observer.

#### Definition of Done
- [ ] Clean API surface.

### P7.03a Adapter Guide

| field | value |
| --- | --- |
| id | P7.03a |
| title | Adapter Guide |
| estimate | 2h |
| status | blocked |
| blocked_by | - |
| blocks | - |
| value | M |

#### Summary

Write ADR/Guide for extension.

#### Steps
1.  Create `docs/ADR-007-adapter-development.md`.
2.  Document how to impl `TelemetrySink` or `AuthProvider` in external crates.

#### Definition of Done
- [ ] Guide exists.

### P7.03b Adapter Template

| field | value |
| --- | --- |
| id | P7.03b |
| title | Adapter Template |
| estimate | 2h |
| status | blocked |
| blocked_by | - |
| blocks | - |
| value | M |

#### Summary

Create a sample adapter.

#### Steps
1.  Add `examples/custom_sink_adapter`.
2.  Show minimal boilerplate.

#### Definition of Done
- [ ] Example compiles.

### P7.04a CoalesceLayer Logic

| field | value |
| --- | --- |
| id | P7.04a |
| title | CoalesceLayer Logic |
| estimate | 2h |
| status | blocked |
| blocked_by | - |
| blocks | - |
| value | H |

#### Summary

Define `CoalesceLayer` struct and Key extraction.

#### Steps
1.  Define `CoalesceLayer<S, KeyFn>`.
2.  Impl `Layer` trait.

#### Definition of Done
- [ ] Struct compiles.

### P7.04b Shared Future Implementation

| field | value |
| --- | --- |
| id | P7.04b |
| title | Shared Future Implementation |
| estimate | 2h |
| status | blocked |
| blocked_by | - |
| blocks | - |
| value | H |

#### Summary

Implement the `SharedFuture` logic to broadcast results.

#### Steps
1.  Use `tokio::sync::broadcast` or `Shared` future.
2.  Manage `HashMap` of in-flight keys.

#### Definition of Done
- [ ] Deduplication logic works.

### P7.04c Coalescing Tests

| field | value |
| --- | --- |
| id | P7.04c |
| title | Coalescing Tests |
| estimate | 2h |
| status | blocked |
| blocked_by | - |
| blocks | - |
| value | M |

#### Summary

Unit tests for singleflight.

#### Steps
1.  Fire 10 concurrent requests.
2.  Assert backend called once.
3.  Assert all 10 get result.

#### Definition of Done
- [ ] Tests pass.
