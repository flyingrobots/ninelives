# Test Plan Boost (coverage to 80% + real sink integrations)

## Phase 1 — Integration Tests with Real Services
- [x] Add `docker-compose.yml` per sink crate (where needed): NATS, Kafka, etcd, Elasticsearch committed under each crate.
  - [x] OTLP collector compose added (otelcol-contrib with logging exporter).
  - [n/a] Prometheus/JSONL don’t require external services.
- [x] Add integration tests per sink crate:
  - NATS, Kafka, etcd, Elastic, Prometheus, JSONL, OTLP all have `tests/integration.rs` and run today. External-service tests are env-gated via `NINE_LIVES_TEST_*` vars instead of `#[ignore]`.
- [ ] Mark tests `#[ignore]` and add a dedicated workflow: decided to keep them runnable by default with env-gates; no separate `integration-sinks.yml`. CI currently runs NATS/Kafka/Elastic/etcd **and OTLP** via `cargo run -p xtask -- it-*`; Prom/JSONL still run in the main `cargo test` job.
- [x] Document per-sink README how to run integration tests:
  - Done for NATS/Kafka/etcd/Elastic/OTLP/Prometheus/JSONL.

## Phase 2 — Coverage to ≥80%
- [x] Add `cargo llvm-cov` tooling; use:
      `cargo llvm-cov --workspace --all-features --exclude xtask --exclude ninelives-nats --exclude ninelives-kafka --exclude ninelives-elastic --exclude ninelives-etcd --exclude ninelives-otlp --exclude ninelives-prometheus --exclude ninelives-jsonl --html --fail-under-lines 80`
- [x] Add GH job (in CI) to run the above and fail under 80% (no sinks/xtask counted).
- [ ] Add targeted tests:
  - `algebra`: edge cases for fallback/fork-join errors
  - `retry`: property-based tests (proptest) for backoff/jitter monotonicity & max-attempts enforcement
  - `bulkhead`: loom tests for permit accounting under contention
  - `timeout`: cancellation/upper-bound coverage
  - `control`: auth/audit/transport validation failures
  - `telemetry`: success/error paths for sinks; drop/evict metrics
  - Companion sinks: unit checks (JSONL file parse, Prometheus counters)
- [ ] Run cookbook examples under `--examples` in CI; add assertions where feasible.

## Phase 3 — Warning Cleanup & Docs
- [ ] Temporarily `#[allow(dead_code)]` on client fields where needed until integration tests land (not applied; most warnings resolved by BYO-client refactor).
- [ ] Add root `docs/TESTING.md` describing unit/integration/coverage/loom workflows (not started).
- [ ] Keep README warning about impending crate split and link to coverage badge once available (warning exists elsewhere, badge pending coverage job).
