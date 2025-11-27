# ADR-008: Control Plane Wire Format (Canonical JSON)

## Status
Accepted

## Context
We need an interoperable wire format so any transport (HTTP, gRPC, JSONL, stdin JSONL) can carry control-plane commands into `CommandRouter`. Earlier ADR-004 defined the transport envelope and trait; this ADR nails down the canonical JSON representation to avoid drift across transports.

## Decision
- Canonical wire object (`TransportEnvelope` JSON):
  ```jsonc
  {
    "id": "<string>",                // unique command id
    "cmd": "<string>",               // command label, e.g., "write_config"
    "args": { ... },                  // arbitrary JSON args for the command
    "auth": {                         // optional auth payload
      "type": "jwt" | "signatures" | "mtls" | "opaque",
      "token": "...",               // when type = jwt
      "payload_hash": "base64?",    // when type = signatures
      "signatures": [
        { "algorithm": "ed25519", "signature": "base64", "key_id": "kid-1" }
      ],
      "peer_dn": "...",             // when type = mtls
      "cert_chain": ["base64-der", ...],
      "data": "base64"              // when type = opaque
    }
  }
  ```
- Responses use `CommandResult` rendered as JSON:
  - `Ack` -> `{ "result": "ack" }`
  - `Value(<string>)` -> `{ "result": "value", "value": "..." }`
  - `List(<array>)` -> `{ "result": "list", "items": [...] }`
  - `Reset` -> `{ "result": "reset" }`
  - `Error(<string>)` -> `{ "result": "error", "message": "..." }`
- All transports MUST be isomorphic to this JSON shape. Binary transports (gRPC) mirror the same fields.
- Field ordering is not significant; UTF-8 encoding is required for text transports.
- Schemas checked into `docs/schemas/transport-envelope.schema.json` and `docs/schemas/command-result.schema.json`.

## Rationale
- Keeps command schema stable across transports and languages.
- Enables tooling (CLI, SDKs) to validate and replay commands easily.
- Auth envelope matches existing `AuthPayload` variants, ensuring a single source of truth.

## Consequences
- Any new auth method must extend both `AuthPayload` and this schema.
- Breaking changes to envelope or result shapes require versioning (future work: add optional `version` field if/when needed).

## Compatibility
- Backward compatible with existing examples; transports already map into `TransportEnvelope`.
- Works with JSONL streams (one envelope per line) for stdin/stdout CLIs.

## Open Questions
- Should we add a `correlation_id` and `timestamp` to the wire by default? (Currently only inside `CommandMeta`/router.)
- Need a formal JSON Schema export in future phases (not required now).
