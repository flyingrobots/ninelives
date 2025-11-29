# Changelog

All notable changes will be documented in this file.

## [Unreleased]

### Added
- **Control Plane**: Added `ninelives::control` module with `CommandRouter`, `AuthRegistry` (JWT/Signature/mTLS hooks), and `CommandHistory`.
- **Circuit Breaker Registry**: Global and local registry support (`ninelives::circuit_breaker_registry`) for managing multiple breakers; `CircuitBreakerLayer::with_registry`.
- **Built-in Commands**: `Set`, `Get`, `List`, `ResetCircuitBreaker`, `GetState`, `ReadConfig`, `WriteConfig`.
- **Dynamic Configuration**: `ConfigRegistry` to expose `Adaptive<T>` values for runtime updates via the control plane.
- **Telemetry**: Added `ninelives::telemetry` with `PolicyEvent`, `LogSink`, `MemorySink`, and `StreamingSink`.
- **Backoff refactor**: concrete strategy types (`Constant`, `Linear`, `Exponential`), `BackoffStrategy` trait, `with_max` validation, helper codes/messages, and monotonic/cap tests.
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
- **Bulkhead**: Fixed potential deadlock in tests regarding service cloning and semaphore permits.
- Removed stray Obsidian files; documentation typos.

## [0.1.0] - Initial

- Initial crate scaffolding with retries, circuit breaker, bulkhead, timeout, stack builder, and helper sleepers/clocks.

[Unreleased]: https://github.com/flyingrobots/ninelives/compare/0.1.0...HEAD
[0.1.0]: https://github.com/flyingrobots/ninelives/releases/tag/0.1.0
