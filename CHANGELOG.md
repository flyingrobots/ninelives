# Changelog

All notable changes will be documented in this file.

## [Unreleased]

- BREAKING: `CommandResult::Error` now wraps a structured `CommandFailure` enum instead of a `String` to support richer error reporting (e.g. `kind`, `message`); update transport serialization and pattern matching accordingly.
- BREAKING: `ForkJoinService::Error` is now `ForkJoinError<E>` to surface both left and right errors on dual failures (previously returned only the left error); update downstream error handling for the new wrapper type. Target release: v0.3.0.
- BREAKING: Added `std::fmt::Debug` trait bound to `S1::Error` in `ForkJoinService` to support improved error diagnostics; ensure custom error types implement `Debug`.
- BREAKING: `ResilienceError<E>` no longer implements `Clone` due to the new `Custom(Box<dyn Error + Send + Sync>)` variant; update call sites/tests that relied on cloning to handle owned/moved errors instead.

### Added

- **Control Plane**: Added `ninelives::control` module with `CommandRouter`, `AuthRegistry` (JWT/Signature/mTLS hooks), and `CommandHistory`.
- **Circuit Breaker Registry**: Global and local registry support (`ninelives::circuit_breaker_registry`) for managing multiple breakers; `CircuitBreakerLayer::with_registry`.
- **Built-in Commands**: `Set`, `Get`, `List`, `ResetCircuitBreaker`, `GetState`, `ReadConfig`, `WriteConfig`.
- **Dynamic Configuration**: `ConfigRegistry` to expose `Adaptive<T>` values for runtime updates via the control plane.
- **Telemetry**: Added `ninelives::telemetry` with `PolicyEvent`, `LogSink`, `MemorySink`, and `StreamingSink`.
- **Backoff**: Introduced concrete strategies (`Constant`, `Linear`, `Exponential`) plus `BackoffStrategy` trait, `with_max` validation, helper codes/messages, monotonic invariant tests, and upper-bound cap tests.
- Decorrelated jitter invariants documented; added concurrent/stateful tests and upper-bound checks.
- Retry builder accepts `Into<Backoff>`; added predicate short-circuit test; zero-attempt validation.
- Timeout: `new_with_max`, improved error message, boundary tests; executable doctests.
- Prelude module re-exporting public API; README rewritten with full API coverage, features table, and test mapping.
- Examples: retry-only, full stack, decorrelated jitter.
- CI: actionlint → fmt → clippy (with `-D missing_docs`) → tests; new docs build job; release-plz config expanded.

### Changed

- Toolchain pinned to `stable`; MSRV documented (1.70).
- rustfmt note on heuristics; gitignore cleaned.

### Fixed

- **Bulkhead**: Fixed test flake: potential deadlock in test setup for bulkhead service clones.
- Removed stray Obsidian files; documentation typos.

## [0.2.0] - 2025-11-25

### Added

- Telemetry sinks with best-effort emission; sink composition (Multicast/Fallback).
- Retry telemetry pipeline (`execute_with_sink`) sharing core logic.
- Bulkhead telemetry reasons (`Saturated` vs `Closed`) and closed event.
- Coverage workflow (`cargo llvm-cov`) and documented local coverage command.
- Adapter changelogs (nats/kafka/otlp/prometheus/jsonl/elastic/etcd/cookbook).

### Changed

- Bulkhead half-open probe counting fixed; closed state distinguishes telemetry.
- Release workflows pin toolchain action commit.
- README roadmap snapshot; examples point to cookbook crate and 0.2 usage.

### Fixed

- release-plz config aligned with non-published adapters; skips adapter publishing.
- Clippy `let-unit-value` warning resolved in telemetry.

## [0.1.0] - Initial

- Initial crate scaffolding with retries, circuit breaker, bulkhead, timeout, stack builder, and helper sleepers/clocks.

[Unreleased]: https://github.com/flyingrobots/ninelives/compare/0.2.0...HEAD
[0.2.0]: https://github.com/flyingrobots/ninelives/compare/0.1.0...0.2.0
[0.1.0]: https://github.com/flyingrobots/ninelives/releases/tag/0.1.0
