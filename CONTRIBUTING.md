# Contributing to Nine Lives

Thanks for wanting to improve the resilience toolkit!

## Quick start
1. Install the latest stable Rust (see <https://rustup.rs/>).
2. Clone and create a branch: `git checkout -b feature/your-idea`.
3. Run the checks locally:
   - `cargo fmt -- --check`
   - `cargo clippy --all-targets --all-features -- -D warnings`
   - `cargo test --all-features --all-targets`
4. Open a PR with a clear description and tests for behavior changes.

## Coding guidelines
- Keep public APIs minimal and well-documented.
- Prefer dependency-free solutions; if a crate is needed, justify it in the PR.
- Keep tests fast; use the provided sleepers/clocks for determinism.

## Commit style
- Conventional commits are appreciated (`feat:`, `fix:`, `chore:`, `docs:`...).

## Releases
- Releases are automated via release-plz, gated by labels `release` **and** `release-ready` on the release PR.
- Monitoring: maintainers watch GitHub Actions workflows (`Release` and `CI`) for failures; enable notifications in repo settings.
- Incident response: if a release job fails, triage the workflow logs, fix forward, and rerun; if a published crate is bad, yank the version on crates.io and cut a follow-up patch release.
- Rollback/manual publish: yanking on crates.io is the rollback path; manual `cargo publish` may be run locally using the `CARGO_REGISTRY_TOKEN` secret if automation is degraded.
