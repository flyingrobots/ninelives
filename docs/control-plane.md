# Control Plane Reference

The Control Plane allows you to inspect and modify the state of a running Nine Lives application.

## Wire Format

All communication uses a canonical JSON envelope.

**Schema Validation:** Enabled by default. Requests violating the schema (e.g., missing fields, wrong types) are rejected immediately.

### Request (`TransportEnvelope`)

```json
{
  "id": "req-uuid-1234",
  "cmd": "write_config",
  "args": {
    "path": "retry.max_attempts",
    "value": "5"
  },
  "auth": {
    "Jwt": { "token": "header.payload.signature" }
  }
}
```

| Field | Type | Description |
|---|---|---|
| `id` | String | Unique request identifier (for correlation). |
| `cmd` | String | Command name (case-insensitive). |
| `args` | Object | Command-specific arguments. |
| `auth` | Object? | Authentication payload (optional depending on provider). |

**Auth Payload Variants:**

* `{"Jwt": {"token": "..."}}`
* `{"Mtls": {"peer_dn": "...", "cert_chain": [[...]]}}`
* `{"Signatures": {"payload_hash": [...], "signatures": [...]}}`
* `{"Opaque": [...]}`

### Response (`CommandResult`)

**Success:**

```json
{
  "result": "ack",
  "id": "req-uuid-1234"
}
```

*Variants:* `ack`, `value` (returns string), `list` (returns array of strings), `reset`.

**Error (`CommandFailure`):**

```json
{
  "result": "error",
  "id": "req-uuid-1234",
  "kind": "not_found",
  "message": "circuit_breaker:api-downstream (not found)"
}
```

*Kinds:* `invalid_args`, `not_found`, `registry_missing`, `internal`.

---

## Built-in Commands

### `Health`

**Args:** None
**Returns:** `Value` (JSON string with status "ok" and version).
**Usage:** Liveness/Readiness probe.

### `GetState`

**Args:** None
**Returns:** `Value` (JSON string).
**Usage:** Full system snapshot (breakers and config).
**Output:**

```json
{
  "breakers": {
    "api-payment": "Closed",
    "api-shipping": "Open"
  },
  "config": {
    "retry.max_attempts": "3",
    "timeout.global": "1000"
  }
}
```

### `WriteConfig`

**Args:** `path` (string), `value` (string)
**Returns:** `Ack`
**Usage:** Update a dynamic configuration value.

### `ReadConfig`

**Args:** `path` (string)
**Returns:** `Value` (current config value)

### `ListConfig`

**Args:** None
**Returns:** `List` (all registered config keys)

### `ResetCircuitBreaker`

**Args:** `id` (string)
**Returns:** `Ack`
**Usage:** Force a circuit breaker to `Closed` state (clears failure counts).

---

## Persistence (Snapshot & Restore)

The Control Plane registries are **in-memory**. To persist configuration changes across restarts:

1. **Snapshot**: Periodically or on shutdown, execute `GetState` and save the `config` object to persistent storage (File, Redis, etc.).
2. **Restore**: On application startup, load the saved config and apply it to the `ConfigRegistry` before starting the service.
