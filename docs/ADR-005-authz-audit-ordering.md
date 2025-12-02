# ADR-005: Authorization vs Audit Layer Ordering

## Status

Accepted

## Decision

Audit every command attempt (including authorization failures) before enforcing authorization.

Layer order: `Audit → AuthZ → Router`

- Authorization denials are recorded with status `denied: <reason>`.
- Successful/errored handler executions are recorded as `ok` or `error: <msg>`.

## Rationale

1) Security: failed/unauthorized attempts must be visible for detection and forensics.
2) Compliance: some regimes require logging all access attempts.
3) Debugging: helps diagnose misconfigurations and unexpected denials.

## Consequences

- Audit volume increases; sinks should be efficient and possibly sampled in the future.
- Auth failures now pass through the audit sink; sinks must be resilient to frequent calls.

## Notes

- Implemented in `CommandRouter::execute`: audit on auth failure and on success/error.
- `MemoryAuditSink` added for tests; `TracingAuditSink` remains for prod logging.
