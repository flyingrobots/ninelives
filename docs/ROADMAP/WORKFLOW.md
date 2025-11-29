# Roadmap Automation (xtask)

Tasks now live inline in `docs/ROADMAP/P#.md` under `## Tasks` (one file per phase). `xtask` reads/writes those sections and keeps the DAG/diagrams in sync.

## Commands
- `cargo run -p xtask --bin xtask sync-dag <PHASE|all>` — imports [`docs/ROADMAP/DAG.csv`](docs/ROADMAP/DAG.csv) (and any per-phase DAGs), recomputes `blocked_by/blocks/status`, rewrites phase files, and regenerates `roadmap.mmd` + `roadmap.svg` (if `mmdc` is installed).
- `cargo run -p xtask --bin xtask suggest [PHASE|all]` — list ready tasks sorted by value/duration and downstream depth.
- `cargo run -p xtask --bin xtask set <TASK_ID> <open|blocked|closed>` — update status and recompute blockers.
- `cargo run -p xtask --bin xtask block <FROM_ID> <TO_ID>` — add dependency edge (FROM blocks TO), recompute blockers/status.
- `cargo run -p xtask --bin xtask add <TASK_ID> <TITLE> <EST> <VALUE> <DEP1,DEP2,...|->` — insert a new task into the appropriate `P#.md`, append edges to the global DAG, and resync. Use `-` for no deps.
- `cargo run -p xtask --bin xtask it-nats|it-kafka|it-etcd|it-elastic|it-otlp` — bring up docker compose (unless env already set) and run sink integration tests.
- `cargo run -p xtask --bin xtask enrich P2` — (legacy) refreshes P2 copy from canned plans.

## Status Marks
- `open` / `blocked` / `closed` (shown as text in the task tables; blocking is recomputed from the DAG).

## Sources of Truth
- Global DAG: [`docs/ROADMAP/DAG.csv`](docs/ROADMAP/DAG.csv) (from,to edges). Per-phase DAGs are optional but supported.
- Phase task lists: `docs/ROADMAP/P#.md` `## Tasks` sections.
- Diagrams: `docs/ROADMAP/roadmap.mmd` (Mermaid) and `docs/ROADMAP/roadmap.svg` (if `mmdc` ran).

## CI / Hooks
- CI: `.github/workflows/roadmap.yml` runs `sync-dag all` and fails on diff. Main CI also runs pre-push-equivalent checks.
- Local hooks are versioned in `scripts/git-hooks/`; run `scripts/setup-hooks.sh` to enable. Pre-push runs fmt, clippy, tests, doc, and `sync-dag all` (blocks on drift). Pre-commit runs fmt+clippy.

## Typical Loop
1) `cargo run -p xtask --bin xtask suggest P3` (or phase/all) to pick a ready task.
2) Follow the workflow below (failing tests first!).
3) `cargo run -p xtask --bin xtask set <TASK_ID> closed`.
4) `cargo run -p xtask --bin xtask sync-dag all`; stage/commit changes (including DAG/mmd/svg).

# Workflow

The project has been planned out in advance. Every task has been spec'd out and arranged in a DAG based on dependencies. It is critical that this DAG be used and kept up-to-date.

## What Should I Work On?

Use `cargo run -p xtask --bin xtask suggest <PHASE|all>` to list ready tasks ordered by value/duration and dependency depth.

## How to Work

1. Start with a clean git state; branch from `main` or the milestone branch.
2. Read your task. **WRITE FAILING TESTS FIRST.** Test behavior, not implementation details or stdout/stderr. Validate the user story. Align with the test plan for your task; if you see gaps, add more tests. Commit.
3. Write the structure for your solution. Commit.
4. Implement the behavior. Commit.
5. Make the tests pass. Commit.
6. Update/add documentation to stay aligned with the code. Commit.
7. `git push`.
8. Open a GitHub PR targeting the correct branch.

## Tips

- USE THE XTASK TOOLS TO KEEP THE DAG UP-TO-DATE.
- Prefer `cargo run -p xtask --bin xtask sync-dag all` before pushing to avoid roadmap drift.

## Adding Cross-Phase Edges
Edit [`docs/ROADMAP/DAG.csv`](docs/ROADMAP/DAG.csv) and run `sync-dag all` to propagate blockers, phase files, and diagrams.
