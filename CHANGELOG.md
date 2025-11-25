# Changelog

All notable changes will be documented in this file.

## [Unreleased]

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

[Unreleased]: https://github.com/flyingrobots/ninelives/compare/0.1.0...HEAD
[0.1.0]: https://github.com/flyingrobots/ninelives/releases/tag/0.1.0
