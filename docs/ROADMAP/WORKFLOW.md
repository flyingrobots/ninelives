# Roadmap Automation (xtask)

We manage tasks/deps via `xtask` instead of manual edits.

## Commands
- `cargo run -p xtask --bin xtask sync-dag <PHASE|all>` — imports `docs/ROADMAP/DAG.csv` (plus any per-phase DAGs), updates `blocked_by/blocks` and status/checklists.
- `cargo run -p xtask --bin xtask suggest [PHASE|all]` — shows ready tasks sorted by value/duration and downstream depth.
- `cargo run -p xtask --bin xtask set <TASK_ID> <open|blocked|closed>` — updates status in frontmatter + checklist, then recomputes blockers.
- `cargo run -p xtask --bin xtask block <FROM_ID> <TO_ID>` — adds directional dependency (FROM blocks TO), recomputes blockers/status.
- `cargo run -p xtask --bin xtask add <TASK_ID> <TITLE> <EST> <VALUE> <DEP1,DEP2,...|->` — creates task file with default sections, appends edges to global DAG, syncs statuses. Use `-` when no deps.
- `cargo run -p xtask --bin xtask enrich P2` — refreshes P2 task bodies from canned plans (value tags, steps, tests). Currently only P2.

## Status Marks
- `[ ]` open
- `[/]` blocked (at least one dependency not closed)
- `[x]` closed

## Sources of Truth
- Global DAG: `docs/ROADMAP/DAG.csv` (from,to edges). Per-phase DAGs are optional; global is preferred.
- Task frontmatter: id, title, estimate, status, blocked_by, blocks, value.

## CI / Hooks
- CI workflow `.github/workflows/roadmap.yml` runs `sync-dag all` and fails on diff.
- Local hook (`.git/hooks/pre-commit`) runs `sync-dag all` and blocks commits on roadmap drift.

## Typical Loop
1) `cargo run -p xtask --bin xtask suggest P2` (or phase/all) to pick next ready task.
2) Implement task.
3) `cargo run -p xtask --bin xtask set <TASK_ID> closed`.
4) `cargo run -p xtask --bin xtask sync-dag all` and stage/commit roadmap changes.

## Adding Cross-Phase Edges
Edit `docs/ROADMAP/DAG.csv`, then `sync-dag all` to propagate blockers/status.
