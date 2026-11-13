# Changelog

All notable changes will be documented in this file.

## [Unreleased]

## [0.2.1] - 2026-11-13

### Security
- tokio 1.48 -> 1.54.1: fixes [RUSTSEC-2026-0386](https://rustsec.org/advisories/RUSTSEC-2026-0386) (`AsyncFd` constructors violate I/O safety). The `~1.48.0` requirement held every dependent on the affected release; tokio is now required as `1.54.1` (any later 1.x), so future tokio fixes reach dependents without a ninelives release.

### Changed
- Dependencies updated within their compatible ranges: tracing 0.1.44, rand 0.9.5, async-trait 0.1.92, futures 0.3.34, tower 0.5.3, tracing-subscriber 0.3.23. rand 0.10 is left for a breaking release because `Jitter::apply_with_rng` exposes rand's `Rng` trait.
- MSRV 1.70 -> 1.88 (tokio 1.54 needs 1.85; hdrhistogram 7.6, via tower's `full` feature, needs 1.88). CI's MSRV job now really builds with 1.88.
- Development toolchain pinned to Rust 1.96.0, matching the shared dev container.

### Fixed
- The release-PR workflow job runs on pushes to `main`; on pull requests it always failed (detached checkout).

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

### Added
- Backoff refactor: concrete strategy types (`Constant`, `Linear`, `Exponential`), `BackoffStrategy` trait, `with_max` validation, helper codes/messages, and monotonic/cap tests.
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
- Removed stray Obsidian files; documentation typos.

## [0.1.0] - Initial

- Initial crate scaffolding with retries, circuit breaker, bulkhead, timeout, stack builder, and helper sleepers/clocks.

[Unreleased]: https://github.com/flyingrobots/ninelives/compare/0.2.0...HEAD
[0.2.0]: https://github.com/flyingrobots/ninelives/compare/0.1.0...0.2.0
[0.1.0]: https://github.com/flyingrobots/ninelives/releases/tag/0.1.0
