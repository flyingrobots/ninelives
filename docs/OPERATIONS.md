# Operations Guide (Health, Validation, Telemetry, DR)

## Health/Readiness

- Use the built-in `BuiltInCommand::Health` exposed through your chosen transport. For HTTP/JSON transports, map it to a `/health` endpoint that executes the command and returns `200 {"status":"ok"}`.
- For readiness that depends on registries, run a `ListConfig`/`GetState` and fail if registries are unavailable.

## Schema Validation

- JSON schema validation is **enabled by default** (`schema-validation` feature). Validation runs on both incoming envelopes and outgoing `CommandResult`s; malformed payloads are rejected before routing.
- To opt out (not recommended), build with `--no-default-features --features arc-swap`.

## Telemetry Wiring

- Prefer centralized, structured sinks: wrap `PolicyEvent` sinks with OTLP/Prom exporters. Example: `MulticastSink::new().with(LogSink::default()).with(StreamingSink::otlp(endpoint))`.
- Ensure sinks are injected into layers (e.g., `RetryLayer::with_sink`, `CircuitBreakerLayer::with_sink`) in your service builder.

## Disaster Recovery / Persistence

- Config and breaker registries are in-memory by default. Implement `ConfigRegistry` / `CircuitBreakerRegistry` with your persistence backend (e.g., database or KV store) and inject via `ControlBuilder::with_config_registry` / `with_circuit_breaker_registry`.
- Snapshot breaker state periodically (e.g., using `snapshot()` on the registry) and store in durable storage; restore on startup before wiring the control plane.
- Use `ConfigRegistry::apply_snapshot` to hydrate configs on startup from your own source (file, Redis, etc.). Pair with `GetState`/`ListConfig` to export before shutdown.
