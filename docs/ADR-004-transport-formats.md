# ADR-004: Control-Plane Transport Formats

## Status
Accepted

## Context

We need a transport-agnostic way to carry control-plane commands (auth, args, ids) across HTTP/gRPC/JSONL/etc. A stable envelope lets us implement multiple transports without coupling to any wire format. The control router already consumes `CommandEnvelope<C>`; we need a canonical wire shape to map to/from it.

## Decision

- Introduce `TransportEnvelope` (serde) with fields: `id: String`, `cmd: String`, `args: serde_json::Value`, `auth: Option<AuthPayload>`.
- Introduce a `Transport` trait with `decode(&self, &[u8]) -> Result<TransportEnvelope, E>`, `encode(&self, &CommandContext, &CommandResult) -> Result<Vec<u8>, E>`, and `map_error(E) -> String`.
- `TransportEnvelope` is the canonical JSON shape; other transports (gRPC, etc.) should be isomorphic to it.
- `AuthPayload` is reused so transports don’t redefine auth representation.

## Rationale

- Keeps control-plane core decoupled from wire protocols.
- Single envelope simplifies testing and fuzzing (one schema).
- Trait allows pluggable transports and consistent error mapping.

## Consequences

- All transports must round-trip through `TransportEnvelope`.
- Additional transports (HTTP, gRPC, JSONL) implement `Transport` and wire conversion.

## Alternatives Considered

- Separate envelope per transport: rejected—adds divergence and duplicative code.
- Serde-tagged enum for multiple envelopes: unnecessary; single shape suffices.

## Notes

- JSON canonical form: `{"id": "...", "cmd": "...", "args": {...}, "auth": {...?}}`.
- Backward compatibility: new transport API lives in `control::transport`; existing router API unchanged.
