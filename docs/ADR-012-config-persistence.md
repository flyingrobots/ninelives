# ADR-012: Config Persistence Strategy

## Decision

Do **not** add built-in file/DB persistence to `ConfigRegistry`. Treat the control plane as an in-memory, runtime intervention layer. Persistence is the host application's responsibility.

## Rationale

- Infra agnosticism: many targets use ephemeral filesystems (Kubernetes, serverless).
- Avoid split-brain between IaC-configured values and locally mutated state.
- Keep scope focused on resilience, not configuration storage.

## Mechanism for Users

- Export: use `GetState` (or `ListConfig`) to snapshot live settings.
- Import: use `ConfigRegistry::apply_snapshot(HashMap<String,String>)` to hydrate at startup from any source (file, Redis, Etcd, etc.).
- Example: `ninelives-cookbook/examples/state_persistence.rs` shows saving to `state.json` and restoring on boot.

## Consequences

- Core remains dependency-free and deterministic.
- Users must wire their own storage, but the API surface makes this trivial.
- Import semantics (apply_snapshot):
  - Unknown keys: ignored; only registered keys are applied.
  - Atomicity: best-effort; collects per-key errors and returns `Err(Vec<String>)` when any key fails to parse/write.
  - Caller guidance: log and surface the failed keys; on startup, consider failing fast if `Err` is returned, or proceed but emit operator guidance (as in `state_persistence.rs`, which would log and report any failed keys).
