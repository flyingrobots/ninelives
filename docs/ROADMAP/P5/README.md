# Phase 5: The Sentinel (Self-Healing Brain)

**Status:** 📋 Planned

## Executive Summary
*   **Story:** Reflexes (Phase 3) handle immediate threats, but complex systems require intelligence. We need a "brain" that can observe long-term trends, execute sophisticated logic, and orchestrate coordinated responses. The Sentinel introduces a scriptable meta-policy engine that acts as an autonomous operator within the system.
*   **Outcome:** A programmable self-healing engine where operators can define complex, high-level reliability rules (e.g., "if region A degrades, shift traffic to B") that run automatically, reducing the need for human intervention during incidents.

## Tasks

### P5.01a Sentinel Crate & Struct

| field | value |
| --- | --- |
| id | P5.01a |
| title | Sentinel Crate & Struct |
| estimate | 2h |
| status | open |
| blocked_by | - |
| blocks | - |
| value | H |

#### Summary

Create `ninelives-sentinel` crate and basic struct.

#### Steps
1.  New crate `ninelives-sentinel`.
2.  Define `Sentinel` struct.
3.  Hold `Arc<TelemetryAggregator>` and `Arc<CommandRouter>`.

#### Definition of Done
- [ ] Crate compiles.

### P5.01b Sentinel Lifecycle

| field | value |
| --- | --- |
| id | P5.01b |
| title | Sentinel Lifecycle |
| estimate | 2h |
| status | blocked |
| blocked_by | - |
| blocks | - |
| value | H |

#### Summary

Implement `start()` and `stop()` for Sentinel.

#### Steps
1.  Implement `start()`: Spawns background task.
2.  Implement `stop()`: Signals cancellation.
3.  Use `tokio_util::sync::CancellationToken`.

#### Definition of Done
- [ ] Sentinel starts and stops cleanly.

### P5.01c Sentinel Tests

| field | value |
| --- | --- |
| id | P5.01c |
| title | Sentinel Tests |
| estimate | 2h |
| status | blocked |
| blocked_by | - |
| blocks | - |
| value | H |

#### Summary

Unit/Integration tests for Sentinel lifecycle.

#### Steps
1.  Test start/stop.
2.  Test graceful shutdown.

#### Definition of Done
- [ ] Tests pass.

### P5.02a Rhai Integration

| field | value |
| --- | --- |
| id | P5.02a |
| title | Rhai Integration |
| estimate | 2h |
| status | blocked |
| blocked_by | - |
| blocks | - |
| value | H |

#### Summary

Add `rhai` dependency and setup basic engine.

#### Steps
1.  Add `rhai = "1.10"`.
2.  Initialize `rhai::Engine`.
3.  Register basic types.

#### Definition of Done
- [ ] Engine compiles and runs "print('hello')".

### P5.02b Sentinel Observe API

| field | value |
| --- | --- |
| id | P5.02b |
| title | Sentinel Observe API |
| estimate | 2h |
| status | blocked |
| blocked_by | - |
| blocks | - |
| value | H |

#### Summary

Expose `get_state()` to Rhai.

#### Steps
1.  Register `get_state` function in Rhai.
2.  Call `telemetry_aggregator.get()`.
3.  Return Map/Struct to script.

#### Definition of Done
- [ ] Script can read metrics.

### P5.02c Sentinel Act API

| field | value |
| --- | --- |
| id | P5.02c |
| title | Sentinel Act API |
| estimate | 2h |
| status | blocked |
| blocked_by | - |
| blocks | - |
| value | H |

#### Summary

Expose `command()` to Rhai.

#### Steps
1.  Register `command` function.
2.  Convert args to `CommandEnvelope`.
3.  Dispatch to `CommandRouter`.

#### Definition of Done
- [ ] Script can issue commands.

### P5.02d Script Sandbox & Validation

| field | value |
| --- | --- |
| id | P5.02d |
| title | Script Sandbox & Validation |
| estimate | 2h |
| status | blocked |
| blocked_by | - |
| blocks | - |
| value | H |

#### Summary

Ensure scripts are safe.

#### Steps
1.  Configure Rhai to disable unsafe features.
2.  Implement `validate_script(str)` function.

#### Definition of Done
- [ ] Unsafe scripts rejected.

### P5.03a Meta-Policy Loop

| field | value |
| --- | --- |
| id | P5.03a |
| title | Meta-Policy Loop |
| estimate | 2h |
| status | blocked |
| blocked_by | - |
| blocks | - |
| value | H |

#### Summary

Implement the loop that runs scripts periodically.

#### Steps
1.  Loop interval (e.g. 5s).
2.  Iterate active scripts.
3.  Run each script.

#### Definition of Done
- [ ] Loop runs scripts.

### P5.03b Script Loader

| field | value |
| --- | --- |
| id | P5.03b |
| title | Script Loader |
| estimate | 2h |
| status | blocked |
| blocked_by | - |
| blocks | - |
| value | H |

#### Summary

Load scripts from directory.

#### Steps
1.  Scan directory for `.rhai` files.
2.  Read and validate.
3.  Add to active list.

#### Definition of Done
- [ ] Scripts loaded from disk.

### P5.03c Hot Reload

| field | value |
| --- | --- |
| id | P5.03c |
| title | Hot Reload |
| estimate | 2h |
| status | blocked |
| blocked_by | - |
| blocks | - |
| value | H |

#### Summary

Implement reload command.

#### Steps
1.  Handle `ReloadMetaPolicy`.
2.  Re-scan directory.
3.  Update active scripts safely.

#### Definition of Done
- [ ] Command updates scripts without restart.

### P5.04a AIMD Script

| field | value |
| --- | --- |
| id | P5.04a |
| title | AIMD Script |
| estimate | 2h |
| status | blocked |
| blocked_by | - |
| blocks | - |
| value | M |

#### Summary

Write AIMD logic in Rhai.

#### Steps
1.  Port AIMD logic to Rhai.
2.  Test with Sentinel.

#### Definition of Done
- [ ] Script works.

### P5.04b Budget Script

| field | value |
| --- | --- |
| id | P5.04b |
| title | Budget Script |
| estimate | 2h |
| status | blocked |
| blocked_by | - |
| blocks | - |
| value | M |

#### Summary

Write Retry Budget logic in Rhai.

#### Steps
1.  Port Budget logic to Rhai.
2.  Test with Sentinel.

#### Definition of Done
- [ ] Script works.

### P5.04c Sentinel Cookbook

| field | value |
| --- | --- |
| id | P5.04c |
| title | Sentinel Cookbook |
| estimate | 2h |
| status | blocked |
| blocked_by | - |
| blocks | - |
| value | M |

#### Summary

Add Sentinel recipes to cookbook.

#### Steps
1.  Create `examples/sentinel_basics.rs`.
2.  Show setup and script loading.

#### Definition of Done
- [ ] Recipe runs.
