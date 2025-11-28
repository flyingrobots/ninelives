# Test Plan Boost (coverage to 80% + real sink integrations)

## Phase 1 — Integration Tests with Real Services
- [ ] Add `docker-compose.yml` per sink crate (under each crate dir):
  - NATS: `nats:latest` (port 4222)
  - Kafka: `redpanda` or `bitnami/kafka` + zookeeper
  - etcd: `quay.io/coreos/etcd` (2379)
  - Elastic: `docker.elastic.co/elasticsearch/elasticsearch:8.x` (single-node)
  - OTLP: `otel/opentelemetry-collector-contrib` (default receivers)
  - Prometheus/JSONL: no external service needed
- [ ] Add integration tests `tests/it.rs` per sink crate using `testcontainers` (preferred) or `USE_COMPOSE=1` fallback:
  - Start service container
  - Instantiate sink with `client` feature
  - Emit a few `PolicyEvent`s
  - Assert delivery/metrics:
    - NATS: subscribe + assert JSON payloads
    - Kafka: consume topic and assert payloads
    - etcd: write/read key under test prefix
    - Elastic: index doc, query back
    - OTLP: collector receives logs (or use in-memory receiver)
    - Prometheus: gather registry, assert counters
    - JSONL: read file lines, assert JSON
- [ ] Mark tests `#[ignore]`; add GH workflow `integration-sinks.yml` to run them with services enabled.
- [ ] Document per-sink README: how to run `cargo test -p <crate> -- --ignored` (optionally `USE_COMPOSE=1`).

## Phase 2 — Coverage to ≥80%
- [ ] Add `cargo llvm-cov` dev dependency/tooling; script/alias to run:
      `cargo llvm-cov --all-features --workspace --exclude xtask --exclude target --html --fail-under-lines 80`
- [ ] Add GH job `coverage.yml` to run the above, upload HTML artifact, fail under 80%.
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
- [ ] Temporarily `#[allow(dead_code)]` on client fields where needed until integration tests land.
- [ ] Add root `docs/TESTING.md` describing unit/integration/coverage/loom workflows.
- [ ] Keep README warning about impending crate split and link to coverage badge once available.
