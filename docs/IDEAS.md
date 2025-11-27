# Ideas & Inspirations

A scratchpad for cool recipes, architectural patterns, and library enhancements that don't yet have a home in the official roadmap but should be captured.

## Cookbook Recipes

### "The Gentle Client"
**Stack**: `RetryLayer` (with Budget) + `BulkheadLayer` (AIMD)
**Concept**: A client that aggressively protects the downstream service. It auto-scales concurrency based on success rate (AIMD) and strictly limits retries to a fixed % of traffic (Retry Budget) to prevent storms.
**Use Case**: High-volume background workers pushing to a fragile API.

### "Hedged Read"
**Stack**: `Policy(CacheRead) & Policy(DbRead)`
**Concept**: Uses the Fork-Join (`&`) operator to race a fast, potentially stale cache against a slower, authoritative database. Returns the first result.
**Use Case**: User-facing dashboards where latency is king.

### "Happy Eyeballs DNS"
**Stack**: `Policy(Ipv4Connect) & Policy(Ipv6Connect)`
**Concept**: Standard RFC 8305 implementation using algebraic composition.
**Use Case**: Network client libraries.

### "Remote Control"
**Stack**: `ninelives-rest` + `CommandRouter`
**Concept**: Expose the internal control plane of a CLI tool via a local HTTP port, allowing another tool (or a web dashboard) to inspect/tune it while it runs.
**Use Case**: Debugging long-running data processing jobs.

## Library Enhancements

### `Policy::from_str` (Declarative Configuration)
**Idea**: Allow parsing a policy stack from a string DSL.
**Example**: `"retry(3, exp) + circuit_breaker(5) + timeout(1s)"`.
**Benefit**: Enables defining resilience strategies in config files (YAML/JSON) without writing Rust code.

### `ForkJoin` Error Preservation
**Idea**: When `A & B` both fail, the error type should ideally preserve *both* errors, not just the last one.
**Implementation**: `ForkJoinError(ErrorA, ErrorB)`.

### `Layer` Prometheus Exporter
**Idea**: A specialized `TelemetrySink` that binds directly to `prometheus-client` metrics, avoiding the overhead of creating intermediate `PolicyEvent` objects for high-throughput scenarios.
**Benefit**: "Zero-allocation" metrics path.

### `tower::Service` for `std::process::Command`
**Idea**: Wrap executing a shell command in a `Service`.
**Benefit**: Apply retries, timeouts, and circuit breakers to shell scripts or external subprocesses. "Resilient Shell Scripting" in Rust.
