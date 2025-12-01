# TODOs for 0.3.0 Release

## P0 – Must fix before release
- [x] Circuit breaker registry: warn on duplicate IDs, keep last registration, add unit test; document “last registration wins” in README + docs/OPERATIONS.
- [x] AuthMode::All: finalize and document merge semantics (principal + merged attributes), add tests; ensure AuthorizationService.poll_ready forwards inner readiness.
- [x] Config/Breaker registries: replace lock `expect` panics with graceful error returns; propagate via CommandError/CommandFailure; add tests.
- [ ] Schema validation docs: add docs/SCHEMA_VALIDATION.md, update README/OPERATIONS to describe `NINELIVES_SCHEMA_VALIDATION` env toggle (default on).
- [ ] Retry: handle `max_attempts == 0` with clear error (already coded) and ensure test passes in CI.

## P1 – High DX/value
- [ ] Control bootstrap helper: add `control::bootstrap_defaults()` wiring router + ChannelTransport with sane defaults; document in README quick start.
- [ ] Telemetry JSON feature: clarify `telemetry-json` feature in README; ensure sinks build with/without it (no-ops or compile-time message).
- [ ] GetState JSON assembly: refactor to helper using `json!`; improve ConfigRegistry error messages (include requested key + available keys).
- [ ] Etcd sink: gate crate behind `etcd-client` feature; ensure CI enables it only when protoc installed.

## P2 – Quality & hygiene
- [ ] Auth aggregation SoC: split authenticate/authorize/aggregate helpers for clarity; tests.
- [ ] Retry loop micro-optimization: preallocate failure buffer, simplify helpers.
- [ ] Elastic sink dependency: add note justifying alpha pin or move to stable 8.x.
- [ ] Logging/telemetry hygiene: document redaction rules; ensure sinks strip/hash sensitive payloads; add a basic redaction test.

## Docs & ops
- [ ] Update README: schema toggle, telemetry-json flag, bootstrap helper snippet.
- [ ] Add ARCHITECTURE.md (layers, control plane, transports) and note feature flags.
- [ ] CONTRIBUTING/DEPENDENCY_VERSIONING: keep npm caret policy note; mention security pin exceptions.
- [ ] SECURITY.md: already added logging/telemetry hygiene—cross-link from README.

## CI
- [ ] Integration tests workflow: ensure protoc install step aligns with `etcd-client` feature; matrix toggles feature off when protoc absent.

## Tests to add/adjust
- [ ] Duplicate breaker ID warning and handle swap.
- [ ] AuthMode::All merged-context behavior.
- [ ] Config/Breaker registry lock error propagation.
- [ ] GetState error handling for Ack/Error cases.
- [ ] Etcd feature off: sinks/tests compile; feature on with protoc passes.
- [ ] Retry max_attempts == 0 path (already present).
