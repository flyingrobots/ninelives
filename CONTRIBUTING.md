# 🤝 Contributing to Nine Lives

Thanks for wanting to improve Nine Lives! This document provides guidelines for contributing code, documentation, and dependencies.

## 🚀 Quick Start

1. Install the latest stable Rust (see <https://rustup.rs/>).
2. Clone and Branch: git checkout -b feature/your-idea.
3. Run Checks Locally:

```bash
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features --all-targets
```
4. Open a PR with a clear description and necessary tests.

## 🧪 Testing Notes

**Placement**: Add unit tests next to the code under `src/`; add integration tests under `tests/` when exercising multiple components together.

**Required Coverage**: Every bug fix needs a regression test; new behavior or configuration paths need happy-path + failure-path coverage; perf/security-sensitive changes need at least one guardrail test.

**Determinism**: Avoid real sleeps and wall-clock reliance. Use the provided testing utilities for time manipulation.

Use the Sleeper utilities (e.g., `InstantSleeper` to skip delays, `TrackingSleeper` to assert calculated waits) and the Clock abstraction (e.g., inject `MonotonicClock` or a manual clock) via constructor/builder injection.

**Example Usage**: For details on available APIs and injection, see the module documentation for the testing utilities, specifically `ninelives::testing::Sleeper` and `ninelives::testing::Clock`. To avoid real delays in a retry test, you would build a policy with `with_sleeper(InstantSleeper)`.

**Local Commands**

```bash
cargo fmt -- --check 
cargo clippy --all-targets --all-features -- -D warnings 
cargo test --all-features --all-targets
```

**CI Expectation**: GitHub Actions runs the same format/clippy/test set; PRs must be green.

**Line Endings**: The repo enforces LF via `.gitattributes` / `.editorconfig`. You should not need to change your local settings. If you are a Windows contributor experiencing CRLF churn despite `.gitattributes`, you can optionally troubleshoot by setting `git config core.autocrlf false` (and optionally `git config core.safecrlf warn`). We recommend leaving defaults unless issues are observed.

## 📝 Coding & Commit Guidelines

**Public APIs**: Keep them minimal and well-documented.

**Dependencies**: Prefer dependency-free solutions; when adding a crate, follow the "Dependency Policy" checklist below.

**Keep Tests Fast**: Use the provided testing sleepers/clocks for determinism.

**Commit Style (Required)**: We require the use of Conventional Commits (`feat:`, `fix:`, `chore:`, `docs:`...) to facilitate automatic release note generation. The format directly feeds into the release notes (see the Releases section for examples).

## 📦 Dependency Policy

Before adding a new crate, you must include the following summary in your PR description:

**Purpose**: What does the crate do?

**Justification**: Why is `std/libcore` insufficient?

**Health**: Maintenance health (recent releases/maintainer activity).

**Security**: Security history (CVE/advisory check).

**Compliance**: License compatibility.

**Impact**: Expected performance/size impact (numbers or rationale).

### Approval

**Sign-off**: The PR requires sign-off from at least one maintainer or designated dependency reviewer before merging.

**SLA**: Expect feedback within two business days. This is a firm guideline; if the deadline is missed, flag the PR again for attention, there is no auto-merge policy.

**Requesting Review**: Flag the PR with the dependency label to request this review.

### Exceptions

**Pre-Approved, Low-Risk Building Blocks**: `serde`, `tokio`, `tracing`, `once_cell`, `anyhow`. Still note their use, but the full justification is not required unless new optional features are enabled.

For deeper criteria and examples, follow this checklist and keep it current; if a dedicated `DEPENDENCY_POLICY.md` is added later, we will link to it here.

## 🚢 Releases

**Automation**: Releases are automated via release-plz and trigger only when both release and release-ready labels are present on the release PR.

**Release-Ready Criteria**: The label may be applied by a maintainer once CI is green and the PR has at least one maintainer approval plus one reviewer approval (or two maintainer approvals). Minor documentation or cosmetic fixes may be exempted by a maintainer.

**Release Notes**: Include a one-line summary in the PR description. `release-plz` compiles release notes from these descriptions. Example: `fix: prevent jitter overflow (closes #123)`.

**Incident Response**: If a Release workflow job fails, triage the logs, fix the root cause on main, and rerun the failed job. If a published crate is bad, yank it on crates.io and cut a follow-up patch release.

### Common Release/CI Failure Modes:

**Auth/Token Issues**: Refresh GitHub/`CARGO_REGISTRY_TOKEN`.

**Network/Transient Runner Failures**: Rerun the job.

**Dependency/Version Conflicts**: Inspect dependency update PRs.

**Permission Errors Publishing**: Verify registry permissions and tokens.

**Rollback/Manual Publish**: Yanking on crates.io is the standard rollback path. Maintainers may run cargo publish locally with `CARGO_REGISTRY_TOKEN`.

## ⚙️ Suggested Local Setup (Optional)

**Recommended**: Enable Actions notifications in personal settings https://github.com/settings/notifications (System → Actions) to track CI status.
