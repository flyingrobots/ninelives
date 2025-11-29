# ADR-003: Minimal Config Command API (Read/Write)

## Status
Proposed

## Context
Phase 2 control plane needs a minimal, transport-agnostic way to read and mutate live configuration (Adaptive handles). Earlier we discussed generic "commands"; for configuration the surface can be two verbs.

## Decision
Expose two core control-plane commands:
- `ReadConfig { path: String }` → returns current value
- `WriteConfig { path: String, value: String }` → applies update to matching Adaptive

Paths map to adaptives registered with the `ConfigRegistry` at runtime. The handler uses this registry to dynamically discover, validate, and update configuration paths.

**Path Discovery & Contract**:
*   **Dynamic Registration**: Paths are registered dynamically at application startup or via configuration by components exposing `Adaptive<T>` handles.
*   **Contract**: The `ConfigRegistry` API (`.read()`, `.write()`, `.contains()`, `.keys()`) provides the programmatic interface for path interaction.
*   **Versioning**: Paths are string-based. New paths can be introduced without breaking old clients. Changes to the semantics or type of existing paths require careful consideration and versioning of the `ConfigRegistry` itself (e.g., through new `register` methods for versioned types).
*   **Client Expectation**: Clients should query `/list_config` (via `BuiltInCommand::ListConfig`) to discover available paths at runtime, rather than relying on hardcoded lists.

Transport-agnostic: HTTP can map to GET/PUT; JSONL/IPC just wrap these payloads.

#### Transport Agnostic Details

The Config Command API (`ReadConfig`/`WriteConfig`) is designed to be transport-agnostic, enabling uniform interaction regardless of the underlying communication protocol (HTTP, gRPC, JSONL, in-process IPC). This section specifies canonical serialization, versioning, and error encoding for consistency across transports.

**1. Serialization**

*   **Value Representation**: Config values (`value` in `WriteConfig`) are always treated as plain UTF-8 strings. The `ConfigRegistry` is responsible for parsing these strings into the appropriate underlying types (`usize`, `Duration`, etc.) via `std::str::FromStr`.
*   **Structured Types**: For structured types like JSON objects or arrays, the string payload is expected to contain the JSON-encoded string. The `ConfigRegistry` will use `serde_json` to parse this string into its internal representation. Transports should ensure `Content-Type: application/json` is used for HTTP/gRPC when structured types are involved.
*   **Schema**: Each config path implicitly defines a schema for its value type (e.g., `usize` must be an integer string, `Duration` a human-readable duration string or milliseconds). Validation against this schema occurs within the `ConfigRegistry` handler.

**2. Versioning and Compatibility**

*   **API Versioning**: The Config Command API adheres to the overall control plane API version. No explicit version header is required for individual config commands. Version compatibility is maintained by ensuring that new versions of the `ConfigRegistry` can still parse paths and values from older clients, or by clearly documenting breaking changes in the ADR/Changelog.
*   **Path Evolution**: New config paths can be introduced in a non-breaking way. Existing paths should not change their semantic meaning or value type without a major version bump or a clear migration path (e.g., deprecating old paths, introducing new ones).
*   **Migration Guarantees**: Clients built against `ninelives` `0.x` API are expected to be compatible with `0.x` servers. Breaking changes to config paths or value types will trigger a major version bump.

**3. Error Encoding per Transport**

Config command errors (e.g., unknown path, parse error, authorization failure) are encapsulated in `CommandResult::Error(String)`. Transports are responsible for mapping this canonical error message to transport-specific error formats and codes.

*   **HTTP/REST**:
    *   `404 Not Found`: For unknown config paths (`CommandResult::Error("unknown config path: ...")`).
    *   `400 Bad Request`: For parse errors (`CommandResult::Error("failed to parse value for path: ...")`) or invalid command format.
    *   `403 Forbidden`: For authorization errors (`CommandError::Auth(...)` when writes are restricted).
    *   `500 Internal Server Error`: For unexpected server-side errors.
    *   **Body**: Error details will be in a JSON body: `{"error": "descriptive message"}`.
        ```http
        HTTP/1.1 404 Not Found
        Content-Type: application/json

        {
          "error": "unknown config path: non_existent_setting"
        }
        ```
*   **gRPC**:
    *   Maps to standard gRPC status codes (e.g., `NOT_FOUND`, `INVALID_ARGUMENT`, `PERMISSION_DENIED`, `INTERNAL`). The `CommandResult::Error` string will be used as the gRPC error message.
*   **JSONL/IPC**:
    *   Errors are returned as `CommandResult::Error(...)` within the standard JSONL command response envelope.
    *   **Example**:
        ```json
        {"id": "cmd-123", "result": "error", "message": "failed to parse value for path: bulkhead.max_concurrent, expected usize"}
        ```

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
