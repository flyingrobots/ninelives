# Architecture Overview

## Core Layout
- **Layers/Policies:** Retry, CircuitBreaker, Bulkhead, Timeout implemented as Tower layers (`src/retry.rs`, `src/circuit_breaker.rs`, `src/bulkhead.rs`, `src/timeout.rs`). Algebraic composition lives in `src/algebra.rs` and is re-exported as `Policy` operators.
- **Telemetry:** `src/telemetry.rs` defines `PolicyEvent` and `TelemetrySink` (a `tower::Service`). JSON helpers are behind the `telemetry-json` feature; sinks opt-in.
- **Control Plane (feature `control`):**
  - Command types and auth: `src/control/types.rs`, `src/control/auth.rs`.
  - Router and handlers: `src/control/router.rs`, `src/control/handler.rs` (built-in commands, config/breaker registries).
  - Transports: `src/control/transport.rs` (generic) and `src/control/transport_channel.rs` (in-process ChannelTransport).
  - Bootstrap helper: `control::bootstrap_defaults()` wires PassthroughAuth + default registries + ChannelTransport for dev use.
- **Registries:** Circuit breaker registry in `src/circuit_breaker_registry.rs`; config registry in `src/control/handler.rs` (ConfigService).

## Feature Flags
- `control`: enables control plane types, transports, JSON schema validation, and related dependencies (`serde`, `serde_json`, `jsonschema`, `base64`).
- `telemetry-json`: enables serde_json dependency and JSON conversion helpers for telemetry (used by sink crates).
- `adaptive-rwlock`, `loom`, `bench-telemetry`: specialized/testing features.

## Schema Validation
- Runtime toggle via `NINELIVES_SCHEMA_VALIDATION` (default on). Applies to `TransportRouter::handle` and `SchemaValidationLayer`. See `docs/SCHEMA_VALIDATION.md`.

## Transports
- **ChannelTransport:** in-process, uses Tokio mpsc to forward `CommandEnvelope` to `CommandRouter`.
- External transports are adapter-specific (e.g., HTTP/JSON) and should map to `TransportEnvelope` and call `TransportRouter::handle` or `SchemaValidationLayer`.

## Telemetry Sinks
- Workspace sinks (nats/kafka/elastic/etcd/jsonl/otlp) depend on `telemetry-json`. Each sink implements `TelemetrySink` and is feature-gated where needed (e.g., etcd-client).

## Control Command Flow
1. Transport decodes to `TransportEnvelope`.
2. Optional schema validation (env-controlled).
3. Conversion to `CommandEnvelope` + `CommandContext` via transport converter.
4. Auth via `AuthRegistry` (mode First/All); authorization; audit/history recorded.
5. Handler executes built-in or custom command.
6. Result optionally schema-validated, then encoded by transport.

## Error/Result Types
- Control errors surface as `CommandError` (non-exhaustive) and `CommandResult`/`CommandFailure` payloads; retry/breaker errors use `ResilienceError`.

## Notes for Extenders
- Prefer composing policies via `Policy` operators; avoid duplicating transport logic—wrap via `TransportRouter` and `SchemaValidationLayer`.
- When adding sinks, enable `telemetry-json` and redact auth/secret fields before emitting events.
