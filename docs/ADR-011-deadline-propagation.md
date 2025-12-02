---
id: ADR-011
title: Deadline Propagation
status: Proposed
---
# ADR-011: Distributed Deadline Propagation

## Context

In a distributed call graph (Service A -> Service B -> Service C), simple timeouts are insufficient.

If Service A has a 5-second timeout and spends 4 seconds processing, it should call Service B with a 1-second timeout. If it calls Service B with a fresh 5-second timeout, the global request might hang for 9+ seconds, wasting resources on work that A's caller has already abandoned.

## Decision

Implement **Deadline Propagation** to respect global time budgets across service boundaries.

### Design

1. **Context**: Define a `DeadlineContext` struct containing `AbsoluteTime` (the instant the request must complete).
2. **Ingress (`DeadlineLayer`)**:
    - Reads headers (e.g., `grpc-timeout`, `X-Request-Deadline`).
    - Calculates absolute deadline.
    - Attaches `DeadlineContext` to the request (via `http::Extensions` or similar).
3. **Enforcement**:
    - Wraps inner service.
    - Before calling inner: `remaining = deadline - now()`.
    - If `remaining <= 0`: Fail immediately (`DeadlineExceeded`).
    - If `remaining > 0`: Enforce local timeout of `min(configured_timeout, remaining)`.
4. **Egress (Transport)**:
    - When making outbound calls, serializes the remaining time into the outbound headers.

### Alternatives Considered

- **Manual passing**: Passing `deadline` as a function argument everywhere. Invasive to user APIs.
- **Local-only timeouts**: Leads to "wasted work" (processing requests that are already dead upstream).

## Consequences

- Requires standardized headers (likely adhering to gRPC or simple HTTP conventions).
- Deeply integrates with the Transport layer (Phase 8).
- Critical for GATOS job chains where total execution time is capped.
