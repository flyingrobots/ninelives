# Phase 3: Adaptive Policies

**Status:** 📋 Planned

## Executive Summary
*   **Story:** Manual tuning doesn't scale. To truly solve the resilience problem, we must connect the system's "eyes" (telemetry) directly to its "hands" (control plane) to create fast, autonomous reflexes. This phase closes the loop, turning static policies into dynamic, self-regulating agents.
*   **Outcome:** A suite of "smart" policies—like AIMD concurrency limits, Rate Limiters, and Retry Budgets—that automatically adapt to changing load conditions to maximize throughput and protect downstream services.

## Tasks

### P3.01a Sliding Window Structure

| field | value |
| --- | --- |
| id | P3.01a |
| title | Sliding Window Structure |
| estimate | 2h |
| status | open |
| blocked_by | - |
| blocks | - |
| value | H |

#### Summary

Implement the core `SlidingWindow` data structure using a ring buffer of atomic counters.

#### Steps
1.  Define `Bucket` struct (timestamp + value).
2.  Define `SlidingWindow` struct (vector of buckets, current index).
3.  Implement `add(value)` logic handling index wrapping.

#### Definition of Done
- [ ] `SlidingWindow` struct exists.
- [ ] Data can be added.
- [ ] Tests verify ring buffer behavior (overwriting old data).

#### Test Plan
- [ ] Unit tests for circular buffer logic.

### P3.01b Window Statistics

| field | value |
| --- | --- |
| id | P3.01b |
| title | Window Statistics |
| estimate | 2h |
| status | blocked |
| blocked_by | - |
| blocks | - |
| value | H |

#### Summary

Implement the statistical methods for `SlidingWindow`: calculating sums, rates, and handling time-based decay.

#### Steps
1.  Implement `sum()`: Iterate valid buckets.
2.  Implement `rate()`: Sum / Window Duration.
3.  Implement `prune()`: Lazy expiry of old buckets during reads.

#### Definition of Done
- [ ] `sum()` and `rate()` implemented.
- [ ] Expired buckets are ignored.

#### Test Plan
- [ ] Unit tests with mocked time (Sleeper/Clock) to verify expiration.

### P3.02a Aggregator Storage

| field | value |
| --- | --- |
| id | P3.02a |
| title | Aggregator Storage |
| estimate | 1.5h |
| status | blocked |
| blocked_by | - |
| blocks | - |
| value | H |

#### Summary

Create the `TelemetryAggregator` struct that holds the map of windowed metrics.

#### Steps
1.  Define `TelemetryAggregator` struct.
2.  Add `Arc<DashMap<String, WindowedMetrics>>`.
3.  Implement `get_metrics(label)` accessor.

#### Definition of Done
- [ ] Struct defined.
- [ ] Thread-safe storage established.

#### Test Plan
- [ ] Concurrent access tests.

### P3.02b TelemetrySink Implementation

| field | value |
| --- | --- |
| id | P3.02b |
| title | TelemetrySink Implementation |
| estimate | 1.5h |
| status | blocked |
| blocked_by | - |
| blocks | - |
| value | H |

#### Summary

Implement the `TelemetrySink` trait for `TelemetryAggregator` to ingest `PolicyEvent`s and update metrics.

#### Steps
1.  Implement `TelemetrySink::record`.
2.  Match event type (Success/Failure).
3.  Update corresponding window in the map.

#### Definition of Done
- [ ] Events update the windows.
- [ ] Error counts and success counts tracked separately.

#### Test Plan
- [ ] Feed events, verify metrics match.

### P3.03a ControlLaw Trait

| field | value |
| --- | --- |
| id | P3.03a |
| title | ControlLaw Trait |
| estimate | 1.5h |
| status | blocked |
| blocked_by | - |
| blocks | - |
| value | H |

#### Summary

Define the generic `ControlLaw` trait interface for adaptive logic.

#### Steps
1.  Define trait `ControlLaw`.
2.  Method: `calculate(metrics, current_val) -> Option<new_val>`.

#### Definition of Done
- [ ] Trait defined.

#### Test Plan
- [ ] N/A (Interface only).

### P3.03b Feedback Loop Runner

| field | value |
| --- | --- |
| id | P3.03b |
| title | Feedback Loop Runner |
| estimate | 1.5h |
| status | blocked |
| blocked_by | - |
| blocks | - |
| value | H |

#### Summary

Implement the async task that runs the feedback loop.

#### Steps
1.  Create `FeedbackLoop` struct.
2.  Implement `run()`: Loop, sleep, get metrics, call law, set handle.

#### Definition of Done
- [ ] Runner loop implemented.

#### Test Plan
- [ ] Run with mock law, verify handle updates.

### P3.04a AIMD Logic

| field | value |
| --- | --- |
| id | P3.04a |
| title | AIMD Logic |
| estimate | 1.5h |
| status | blocked |
| blocked_by | - |
| blocks | - |
| value | M |

#### Summary

Implement the AIMD (Additive Increase, Multiplicative Decrease) math as a `ControlLaw`.

#### Steps
1.  Implement `AimdLaw` struct.
2.  Implement `calculate()`: +1 on success, *0.5 on overload.

#### Definition of Done
- [ ] Math logic implemented correctly.

#### Test Plan
- [ ] Unit tests for the math.

### P3.04b AIMD Integration

| field | value |
| --- | --- |
| id | P3.04b |
| title | AIMD Integration |
| estimate | 1.5h |
| status | blocked |
| blocked_by | - |
| blocks | - |
| value | M |

#### Summary

Connect `AimdLaw` to a `Bulkhead` using the Feedback Loop.

#### Steps
1.  Create example wiring `Bulkhead` + `AimdLaw`.
2.  Verify end-to-end behavior.

#### Definition of Done
- [ ] Cookbook recipe / integration test.

#### Test Plan
- [ ] Integration test simulating load.

### P3.05a Retry Budget Logic

| field | value |
| --- | --- |
| id | P3.05a |
| title | Retry Budget Logic |
| estimate | 1.5h |
| status | blocked |
| blocked_by | - |
| blocks | - |
| value | M |

#### Summary

Implement the Retry Budget math as a `ControlLaw`.

#### Steps
1.  Implement `RetryBudgetLaw`.
2.  Logic: If retry_ratio > target, return 0 (disable retries).

#### Definition of Done
- [ ] Logic implemented.

#### Test Plan
- [ ] Unit tests for ratio calculation.

### P3.05b Retry Budget Integration

| field | value |
| --- | --- |
| id | P3.05b |
| title | Retry Budget Integration |
| estimate | 1.5h |
| status | blocked |
| blocked_by | - |
| blocks | - |
| value | M |

#### Summary

Connect `RetryBudgetLaw` to a `RetryPolicy` using the Feedback Loop.

#### Steps
1.  Create example wiring.
2.  Verify end-to-end behavior.

#### Definition of Done
- [ ] Cookbook recipe / integration test.

#### Test Plan
- [ ] Integration test simulating retry storm.

### P3.06 System State Query Interface

| field | value |
| --- | --- |
| id | P3.06 |
| title | System State Query Interface |
| estimate | 2h |
| status | blocked |
| blocked_by | - |
| blocks | - |
| value | M |

#### Summary

Expose the aggregated metrics from P3.02 via the Control Plane.

#### Context

Extends the "Simple State Query" from P2.11 to include rich windowed metrics. This is the "dashboard" data (Success Rate, Throughput, etc.).

#### Steps
1.  **Command**: Enhance `GetState` (or add `GetMetrics`) to pull from `TelemetryAggregator`.
2.  **Schema**:
    ```json
    {
      "services": {
        "s3": {
          "success_rate": 0.99,
          "throughput_rps": 50.0,
          "p99_latency_ms": 120
        }
      }
    }
    ```

#### Definition of Done
- [ ] `TelemetryAggregator` exposes a snapshot method.
- [ ] Control plane command returns the snapshot.

#### Test Plan
- [ ] **Integration**: Generate load, query metrics, verify numbers match reality.

### P3.07a RateLimitLayer Struct

| field | value |
| --- | --- |
| id | P3.07a |
| title | RateLimitLayer Struct |
| estimate | 2h |
| status | blocked |
| blocked_by | - |
| blocks | - |
| value | H |

#### Summary

Implement `RateLimitLayer` using `governor` or Token Bucket.

#### Steps
1.  Add `governor` dependency.
2.  Define `RateLimitLayer<S>`.
3.  Implement `Service::call`: Check quota, await readiness or error.

#### Definition of Done
- [ ] Layer enforces RPS limit.

### P3.07b Adaptive Quota Integration

| field | value |
| --- | --- |
| id | P3.07b |
| title | Adaptive Quota Integration |
| estimate | 2h |
| status | blocked |
| blocked_by | - |
| blocks | - |
| value | H |

#### Summary

Connect `RateLimitLayer` to `Adaptive<u32>`.

#### Steps
1.  Expose quota as `Adaptive<u32>`.
2.  On config change, update `governor` limiter.

#### Definition of Done
- [ ] Live tuning of RPS works.

### P3.07c Rate Limiting Recipe

| field | value |
| --- | --- |
| id | P3.07c |
| title | Rate Limiting Recipe |
| estimate | 2h |
| status | blocked |
| blocked_by | - |
| blocks | - |
| value | M |

#### Summary

Add Rate Limiting example to cookbook.

#### Steps
1.  `examples/rate_limiting.rs`.
2.  Demonstrate throttling.

#### Definition of Done
- [ ] Example runs.
