# Phase 6: Shadow Evaluation

**Status:** 📋 Planned

## Executive Summary
*   **Story:** Change is the leading cause of outages. Rolling out a new resilience policy shouldn't be a leap of faith. We introduce "Shadow Mode"—the ability to run a new policy configuration alongside the live one, processing real traffic without affecting the result, to prove it works before it goes live.
*   **Outcome:** Risk-free policy evolution. Operators can "what-if" test aggressive configurations (e.g., tighter timeouts, stricter rate limits) in production, verify their safety via shadow telemetry, and automatically promote them when proven stable.

## Tasks

### P6.01a ShadowLayer Struct

| field | value |
| --- | --- |
| id | P6.01a |
| title | ShadowLayer Struct |
| estimate | 2h |
| status | open |
| blocked_by | - |
| blocks | - |
| value | H |

#### Summary

Define the `ShadowLayer` struct and implement the basic `tower::Layer` and `Service` traits. It should hold two services: `primary` and `shadow`.

#### Steps
1.  Define `ShadowLayer<A, B>`.
2.  Implement `Service::call`: Clone request (requires `Clone`), call both services.
3.  Return result from `primary`.

#### Definition of Done
- [ ] `ShadowLayer` wraps two services.
- [ ] Returns primary result.

### P6.01b Adaptive Shadow Support

| field | value |
| --- | --- |
| id | P6.01b |
| title | Adaptive Shadow Support |
| estimate | 2h |
| status | blocked |
| blocked_by | - |
| blocks | - |
| value | H |

#### Summary

Extend `Adaptive<T>` to support a separate "shadow" value alongside the "primary" value.

#### Steps
1.  Modify `Adaptive<T>` internal storage (e.g., `ArcSwap<ConfigState>`).
2.  Add `set_shadow(val)` and `get_shadow()` methods.
3.  Ensure lock-free access.

#### Definition of Done
- [ ] `Adaptive` holds two values.
- [ ] Shadow value can be read independently.

### P6.01c Shadow Isolation

| field | value |
| --- | --- |
| id | P6.01c |
| title | Shadow Isolation |
| estimate | 2h |
| status | blocked |
| blocked_by | - |
| blocks | - |
| value | H |

#### Summary

Ensure shadow execution is isolated and does not block the primary path.

#### Steps
1.  Use `tokio::spawn` (or similar) for the shadow future.
2.  Ensure panic in shadow doesn't crash primary.
3.  Add basic error handling for shadow path.

#### Definition of Done
- [ ] Shadow runs concurrently/detached.
- [ ] Primary path unaffected by shadow panic.

### P6.01d ShadowLayer Unit Tests

| field | value |
| --- | --- |
| id | P6.01d |
| title | ShadowLayer Unit Tests |
| estimate | 2h |
| status | blocked |
| blocked_by | - |
| blocks | - |
| value | M |

#### Summary

Unit tests verifying request cloning and response routing.

#### Steps
1.  Test: Primary returns OK, Shadow returns Err -> Result is OK.
2.  Test: Primary returns Err -> Result is Err.
3.  Test: Request matches on both sides.

#### Definition of Done
- [ ] Basic routing logic verified.

### P6.01e ShadowLayer Integration

| field | value |
| --- | --- |
| id | P6.01e |
| title | ShadowLayer Integration |
| estimate | 2h |
| status | blocked |
| blocked_by | - |
| blocks | - |
| value | M |

#### Summary

Integration test with real layers (e.g. Retry).

#### Steps
1.  Wrap `RetryLayer` (Primary: 3 attempts) and `RetryLayer` (Shadow: 1 attempt).
2.  Simulate failure.
3.  Verify behavior differences.

#### Definition of Done
- [ ] Shadow works with real layers.

### P6.02a ShadowEvent Definition

| field | value |
| --- | --- |
| id | P6.02a |
| title | ShadowEvent Definition |
| estimate | 1.5h |
| status | blocked |
| blocked_by | - |
| blocks | - |
| value | H |

#### Summary

Define `ShadowEvent` or extend `PolicyEvent` to differentiate shadow traffic.

#### Steps
1.  Add `PolicyEvent::Shadow { inner: Box<PolicyEvent> }` or similar.
2.  Or add metadata context to `PolicyEvent`.

#### Definition of Done
- [ ] Event type supports shadow distinction.

### P6.02b Shadow Emission

| field | value |
| --- | --- |
| id | P6.02b |
| title | Shadow Emission |
| estimate | 1.5h |
| status | blocked |
| blocked_by | - |
| blocks | - |
| value | H |

#### Summary

Update `ShadowLayer` to emit events.

#### Steps
1.  Capture shadow result.
2.  Emit `ShadowEvent`.

#### Definition of Done
- [ ] Shadow path emits events.

### P6.02c Aggregator Shadow Support

| field | value |
| --- | --- |
| id | P6.02c |
| title | Aggregator Shadow Support |
| estimate | 1.5h |
| status | blocked |
| blocked_by | - |
| blocks | - |
| value | H |

#### Summary

Update `TelemetryAggregator` to separate shadow metrics.

#### Steps
1.  Check event type.
2.  If shadow, use "label-shadow" key.
3.  Store separately from primary.

#### Definition of Done
- [ ] Aggregator splits metrics.

### P6.02d Metrics Separation Tests

| field | value |
| --- | --- |
| id | P6.02d |
| title | Metrics Separation Tests |
| estimate | 1.5h |
| status | blocked |
| blocked_by | - |
| blocks | - |
| value | M |

#### Summary

Verify primary and shadow metrics don't mix.

#### Steps
1.  Generate traffic.
2.  Assert `primary_errors != shadow_errors`.

#### Definition of Done
- [ ] Separation verified.

### P6.03a Atomic Swap Logic

| field | value |
| --- | --- |
| id | P6.03a |
| title | Atomic Swap Logic |
| estimate | 2h |
| status | blocked |
| blocked_by | - |
| blocks | - |
| value | H |

#### Summary

Implement `Adaptive<T>::promote_shadow()` to atomically move shadow value to primary.

#### Steps
1.  Implement `compare_exchange` loop or similar on internal state.
2.  Ensure consistency (no partial updates).

#### Definition of Done
- [ ] `promote_shadow` works atomically.

### P6.03b Promotion Command

| field | value |
| --- | --- |
| id | P6.03b |
| title | Promotion Command |
| estimate | 2h |
| status | blocked |
| blocked_by | - |
| blocks | - |
| value | H |

#### Summary

Add `PromoteShadow` command to Control Plane.

#### Steps
1.  Add `BuiltInCommand::PromoteShadow { path }`.
2.  Wire to handler logic calling `promote_shadow()`.

#### Definition of Done
- [ ] Command promotes the value.

### P6.03c Promotion Meta-Policy

| field | value |
| --- | --- |
| id | P6.03c |
| title | Promotion Meta-Policy |
| estimate | 2h |
| status | blocked |
| blocked_by | - |
| blocks | - |
| value | M |

#### Summary

Write a Sentinel script to auto-promote.

#### Steps
1.  Script compares primary vs shadow metrics.
2.  If shadow better/stable, issue `PromoteShadow`.

#### Definition of Done
- [ ] Script logic correct.

### P6.03d End-to-End Promotion

| field | value |
| --- | --- |
| id | P6.03d |
| title | End-to-End Promotion |
| estimate | 2h |
| status | blocked |
| blocked_by | - |
| blocks | - |
| value | M |

#### Summary

Integration test for the full loop.

#### Steps
1.  Set up ShadowLayer + Sentinel.
2.  Simulate better shadow performance.
3.  Wait for promotion.

#### Definition of Done
- [ ] Auto-promotion verified.

### P6.04a Safety ADR

| field | value |
| --- | --- |
| id | P6.04a |
| title | Safety ADR |
| estimate | 2h |
| status | blocked |
| blocked_by | - |
| blocks | - |
| value | M |

#### Summary

Document safety guarantees.

#### Steps
1.  Write `docs/ADR-006-shadow-safety.md`.
2.  Cover latency isolation, panic safety, atomic promotion.

#### Definition of Done
- [ ] ADR exists.

### P6.04b Shadow Cookbook

| field | value |
| --- | --- |
| id | P6.04b |
| title | Shadow Cookbook |
| estimate | 2h |
| status | blocked |
| blocked_by | - |
| blocks | - |
| value | M |

#### Summary

Add "Safe Shadowing" recipe.

#### Steps
1.  Create `examples/safe_shadowing.rs`.
2.  Demonstrate Shadow + Bulkhead + Sentinel.

#### Definition of Done
- [ ] Recipe runs.
