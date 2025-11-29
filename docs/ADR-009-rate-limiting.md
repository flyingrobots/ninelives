---
id: ADR-009
title: Adaptive Rate Limiting
status: Proposed
---
# ADR-009: Adaptive Rate Limiting

## Context
`ninelives` currently provides a `BulkheadLayer` which limits **concurrency** (N active requests). However, it lacks a mechanism to limit **throughput** (N requests per second).

In distributed systems like GATOS, downstream services (e.g., a database or Git remote) often have throughput limits independent of concurrency. For example, a service might accept 100 concurrent connections but only process 50 writes/sec. Without a rate limiter, a client can flood the service with short, fast requests that overwhelm its write capacity.

Furthermore, this limit needs to be **adaptive**. A static limit of 50 RPS is too brittle; if the service degrades, we should dynamically throttle back to 20 RPS.

## Decision
Implement a `RateLimitLayer` based on the **Token Bucket** algorithm (specifically the "Generic Cell Rate Algorithm" or GCRA), integrated with the `ninelives` adaptive configuration system.

### Design
1.  **Dependency**: Use the `governor` crate (standard for Rust rate limiting) or implement a lightweight GCRA if `no_std` is required.
2.  **Layer**: `RateLimitLayer<S>`.
3.  **Configuration**:
    - `quota: Adaptive<u32>`: Requests per second.
    - `burst: Adaptive<u32>`: Burst capacity.
4.  **Adaptive Integration**:
    - The `quota` handle allows the Sentinel (Phase 5) to tune throughput dynamically.
    - e.g., If `TelemetryAggregator` sees 503s, Sentinel writes `quota = current * 0.8`.

### Alternatives Considered
- **Leaky Bucket**: Simpler but less flexible with bursts.
- **Fixed Window**: Inaccurate at window boundaries (thundering herd).
- **Bulkhead Only**: Insufficient for throughput constraints (high RPS, low latency).

## Consequences
- Adds a dependency on the `governor` crate. This crate implements a robust Token Bucket algorithm and will be the chosen dependency.
- Enables clients to shed low-priority requests under load, improving system stability.
- Complements Bulkhead: `RateLimitLayer` primarily protects CPU/IOPS by controlling request rate, while `BulkheadLayer` protects memory/sockets by limiting concurrent operations.
- **Composition**: When composing `RateLimitLayer` with `BulkheadLayer`, it is recommended to place `RateLimitLayer` *before* `BulkheadLayer` in the Tower stack (i.e., `RateLimitLayer + BulkheadLayer`). This ensures that excess load is shed by the rate limiter *before* requests consume valuable concurrency slots from the bulkhead, optimizing resource utilization.
