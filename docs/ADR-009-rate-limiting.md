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
- Adds a dependency on `governor` (or equivalent logic).
- Enables "Shed Load" patterns in Phase 9.
- Complements Bulkhead: Bulkhead protects memory/sockets; RateLimit protects CPU/IOPS.
