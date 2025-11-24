# Contributing to Nine Lives

Thanks for wanting to improve Nine Lives!

## Quick start
1. Install the latest stable Rust (see <https://rustup.rs/>).
2. Clone and create a branch: `git checkout -b feature/your-idea`.
3. Run the checks locally:
   - `cargo fmt -- --check`
   - `cargo clippy --all-targets --all-features -- -D warnings`
   - `cargo test --all-features --all-targets`
4. Open a PR with a clear description and tests: required for bug fixes, refactors that could affect behavior, new features, and perf/security-sensitive changes. See the testing notes below for how and where to add them.

## Coding guidelines
- Keep public APIs minimal and well-documented.
- Prefer dependency-free solutions; if a crate is needed, justify it in the PR.
- Keep tests fast; use the provided testing sleepers/clocks for determinism (see `src/sleeper.rs` for `InstantSleeper`/`TrackingSleeper` and `src/clock.rs` for `MonotonicClock`). Example: inject `InstantSleeper` into retry/bulkhead tests to avoid real delays.

## Commit style
- Conventional commits are appreciated (`feat:`, `fix:`, `chore:`, `docs:`...).

## Releases
- Releases are automated via release-plz, gated by labels `release` **and** `release-ready` on the release PR.
- Bump notes: include a one-line summary in the PR description; release-plz compiles release notes from PR descriptions (or CHANGELOG entries if added). Example: "fix: prevent jitter overflow (closes #123)".
- Monitoring: maintainers watch GitHub Actions workflows (`Release` and `CI`) for failures; enable Actions notifications in personal settings <https://github.com/settings/notifications> (System → Actions). Team-wide alerts may need shared accounts or external alerting.
- Incident response: if a release job fails, triage the workflow logs, fix the root cause on `main`, and rerun; if a published crate is bad, yank the version on crates.io and cut a follow-up patch release.
- Rollback/manual publish: yanking on crates.io is the rollback path; maintainers may run `cargo publish` locally with `CARGO_REGISTRY_TOKEN` (see <https://doc.rust-lang.org/cargo/reference/registry-authentication.html>) if automation is degraded.
