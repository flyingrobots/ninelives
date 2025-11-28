# Roadmap Automation (xtask)

Tasks now live inline in `docs/ROADMAP/P#.md` under `## Tasks` (one file per phase). `xtask` reads/writes those sections and keeps the DAG/diagrams in sync.

## Commands
- `cargo run -p xtask --bin xtask sync-dag <PHASE|all>` — imports [`docs/ROADMAP/DAG.csv`](docs/ROADMAP/DAG.csv) (and any per-phase DAGs), recomputes `blocked_by/blocks/status`, rewrites phase files, and regenerates `roadmap.mmd` + `roadmap.svg` (if `mmdc` is installed).
- `cargo run -p xtask --bin xtask suggest [PHASE|all]` — list ready tasks sorted by value/duration and downstream depth.
- `cargo run -p xtask --bin xtask set <TASK_ID> <open|blocked|closed>` — update status and recompute blockers.
- `cargo run -p xtask --bin xtask block <FROM_ID> <TO_ID>` — add dependency edge (FROM blocks TO), recompute blockers/status.
- `cargo run -p xtask --bin xtask add <TASK_ID> <TITLE> <EST> <VALUE> <DEP1,DEP2,...|->` — insert a new task into the appropriate `P#.md`, append edges to the global DAG, and resync. Use `-` for no deps.
- `cargo run -p xtask --bin xtask it-nats` — bring up NATS via docker compose and run NATS integration tests (uses env var if provided).
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
2) Implement.
3) `cargo run -p xtask --bin xtask set <TASK_ID> closed`.
4) `cargo run -p xtask --bin xtask sync-dag all`; stage/commit changes (including DAG/mmd/svg).

## Adding Cross-Phase Edges
Edit [`docs/ROADMAP/DAG.csv`](docs/ROADMAP/DAG.csv) and run `sync-dag all` to propagate blockers, phase files, and diagrams.
