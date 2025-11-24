# Contributing to Nine Lives

Thanks for wanting to improve Nine Lives!

## Quick start
1. Install the latest stable Rust (see <https://rustup.rs/>).
2. Clone and create a branch: `git checkout -b feature/your-idea`.
3. Run the checks locally:
   - `cargo fmt -- --check`
   - `cargo clippy --all-targets --all-features -- -D warnings`
   - `cargo test --all-features --all-targets`
4. Open a PR with a clear description and tests: required for bug fixes, refactors that could affect behavior, new features, and perf/security-sensitive changes. See “Testing notes” below for what to add and where.

## Testing notes
- Add unit tests next to the code under `src/`; add integration tests under `tests/` when exercising multiple components together.
- Required coverage: every bug fix needs a regression test; new behavior or configuration paths need happy-path + failure-path coverage; perf/security-sensitive changes need at least one guardrail test.
- Determinism: avoid real sleeps and wall-clock reliance. Use the `Sleeper` test utilities (e.g., `InstantSleeper` to skip delays, `TrackingSleeper` to assert calculated waits) and the `Clock` abstraction (e.g., inject `MonotonicClock` or a manual clock) via constructor/builder injection.
- Examples:
  - Retry backoff without waiting: build a policy with `with_sleeper(InstantSleeper)` and assert the number of attempts (see retry policy tests in `src/retry.rs`).
  - Assert computed delays: use `TrackingSleeper` to capture per-attempt waits and compare against expected backoff/jitter ranges.
  - Time-driven logic: pass a manual clock implementing `Clock` into the circuit breaker to advance time without sleeping (see circuit breaker tests).
- Commands to run locally: `cargo fmt -- --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test --all-features --all-targets`.
- CI expectation: GitHub Actions runs the same format/clippy/test set; PRs must be green.
- Keep this section in sync with exported module docs rather than file paths to avoid stale references.
- Line endings: the repo enforces LF via `.gitattributes`/`.editorconfig`; set `git config core.autocrlf false` and optionally `git config core.safecrlf warn` locally to avoid CRLF churn.

## Coding guidelines
- Keep public APIs minimal and well-documented.
- Prefer dependency-free solutions; when adding a crate, follow the “Dependency policy” checklist below and include the summary in your PR.
- Keep tests fast; use the provided testing sleepers/clocks for determinism (see module docs for `Sleeper` utilities and `Clock`). Example: inject a test sleeper into retry/bulkhead tests to avoid real delays.

## Dependency policy
- Before adding a crate, include in the PR description: purpose, why std/libcore is insufficient, maintenance health (recent releases/maintainer activity), security history (CVE/advisory check), license compatibility, and expected performance/size impact (numbers or rationale).
- Approval: a maintainer or designated dependency reviewer must sign off; expect feedback within two business days. Flag the PR with `dependency` label to request review.
- Exceptions (pre-approved, low-risk building blocks): `serde`, `tokio`, `tracing`, `once_cell`, `anyhow`. Still note their use, but the full justification is not required unless new optional features are enabled.
- For deeper criteria and examples, follow this checklist and keep it current; if a dedicated DEPENDENCY_POLICY.md is added later, link to it here.

## Commit style
- Conventional commits are appreciated (`feat:`, `fix:`, `chore:`, `docs:`...).

## Releases
- Releases are automated via release-plz and trigger only when both `release` and `release-ready` labels are present on the release PR.
- Release-ready label criteria: may be applied by a maintainer once CI is green, the PR has required approvals, the PR description includes a concise change summary, and any user-facing changes are noted for release notes.
- Version bump notes: include a one-line summary in the PR description; release-plz compiles release notes from PR descriptions (or CHANGELOG entries if added). Example: "fix: prevent jitter overflow (closes #123)".
- Incident response: if a Release workflow job fails, triage the logs, fix the root cause on `main`, and rerun the failed job in the Release workflow (see GitHub rerun docs: <https://docs.github.com/en/actions/managing-workflow-runs/re-running-workflows-and-jobs>); if a published crate is bad, yank it on crates.io and cut a follow-up patch release.
- Common release/CI failure modes: auth/token issues (refresh GitHub/CARGO_REGISTRY_TOKEN), network/transient runner failures (rerun job), dependency or version conflicts (inspect dependency update PRs), permission errors publishing (verify registry permissions and tokens).
- Rollback/manual publish: yanking on crates.io is the rollback path; maintainers may run `cargo publish` locally with `CARGO_REGISTRY_TOKEN` (see <https://doc.rust-lang.org/cargo/reference/registry-authentication.html>). Set the token as an env var before publishing (e.g., `export CARGO_REGISTRY_TOKEN=your_token`); never commit tokens—store them in your OS keychain locally or as GitHub Actions secrets with minimal scopes and rotate regularly.

### Suggested local setup
- Enable Actions notifications in personal settings <https://github.com/settings/notifications> (System → Actions). Team-wide alerts may need shared accounts or external alerting.
