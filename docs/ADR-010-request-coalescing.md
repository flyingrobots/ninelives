---
id: ADR-010
title: Request Coalescing (Singleflight)
status: Proposed
---
# ADR-010: Request Coalescing (Singleflight)

## Context
In read-heavy workloads (like the GATOS State Plane or Ledger Plane), "cache stampedes" or "thundering herds" occur when many clients request the same resource (e.g., the same Git object ID or policy result) simultaneously.

Without intervention, 1,000 concurrent requests for `Object X` result in 1,000 fetches to the backend. Ideally, we should send **one** fetch and have all 1,000 clients await that single result.

## Decision
Implement a `CoalesceLayer` (often called "Singleflight" in Go) that deduplicates in-flight requests based on a key.

### Design
1.  **Layer**: `CoalesceLayer<S, KeyFn>`.
2.  **Key Extraction**: A closure `Fn(&Request) -> Key` determines uniqueness (e.g., URL, Object ID).
3.  **Mechanism**:
    - Maintain a `HashMap<Key, SharedFuture>`.
    - On request:
        - If key exists in map: Attach to the existing future.
        - If key missing: Call inner service, insert future into map.
    - On completion: Remove from map, broadcast result to all waiters.
4.  **Constraints**:
    - Response must be `Clone`.
    - Request must be hashable (or mappable to a hashable key).

### Alternatives Considered
- **Caching**: `CacheLayer` stores results *after* they complete. `CoalesceLayer` deduplicates *during* execution. They are complementary.
- **Client-side locking**: Hard to coordinate in distributed systems.

## Consequences
- Drastically reduces load on backends for hot keys.
- Increases latency tail for the "leader" request (since it bears the actual work), but improves global throughput.
- Requires `Response: Clone`.
