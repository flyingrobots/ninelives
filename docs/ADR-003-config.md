# ADR-003: Minimal Config Command API (Read/Write)

## Status
Proposed

## Context
Phase 2 control plane needs a minimal, transport-agnostic way to read and mutate live configuration (Adaptive handles). Earlier we discussed generic "commands"; for configuration the surface can be two verbs.

## Decision
Expose two core control-plane commands:
- `ReadConfig { path: String }` → returns current value
- `WriteConfig { path: String, value: String }` → applies update to matching Adaptive

Paths map to known adaptives (e.g., `retry.max_attempts`, `timeout.duration`, `bulkhead.max_concurrent`, `circuit.failure_threshold`, `circuit.recovery_timeout`, etc.). Handler validates path, parses value, updates Adaptive, and returns the effective value.

Transport-agnostic: HTTP can map to GET/PUT; JSONL/IPC just wrap these payloads.

## Rationale
- Smallest orthogonal surface: two verbs cover config I/O.
- Clear intent (configuration, not arbitrary actions), easier to secure and audit.
- Works uniformly across transports; easy to extend with more paths over time.

## Consequences
- Command router needs a config registry mapping paths → Adaptive handles and parsers.
- Authorization can be applied per path (e.g., restrict writes in prod).
- Bulkhead currently only grows capacity; shrinking is documented/unsupported until implemented.

## Open Questions
- Do we allow batch writes/reads? (future extension)
- Should paths be enums in code with string serialization to avoid typos? (likely yes)
- Shrink semantics for bulkhead semaphore: document as unsupported or implement safe shrink.
