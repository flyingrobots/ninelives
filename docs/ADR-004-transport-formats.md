# ADR-004: Control-Plane Transport Formats

## Status

Accepted

## Context

We need a transport-agnostic way to carry control-plane commands (auth, args, ids) across HTTP/gRPC/JSONL/etc. A stable envelope lets us implement multiple transports without coupling to any wire format. The control router already consumes `CommandEnvelope<C>`; we need a canonical wire shape to map to/from it.

## Decision

A clear separation is established between the wire-format `TransportEnvelope` and the internal router-facing `CommandEnvelope<C>`.

### **1. Wire Format (`TransportEnvelope`)**

- Introduce `TransportEnvelope` (serde-deriveable) as the canonical, transport-agnostic wire-format.

    ```rust
    pub struct TransportEnvelope {
        pub id: String,                     // Opaque command identifier
        pub cmd: String,                    // Command label/name (e.g., "write_config")
        pub args: serde_json::Value,        // Arbitrary JSON args for the command
        pub auth: Option<AuthPayload>,      // Optional auth payload
    }
    ```

- `TransportEnvelope` is a concrete, non-generic struct, designed for easy serialization to/from byte streams (e.g., JSON).

### **2. Internal Router Format (`CommandEnvelope<C>`)**

- The `CommandRouter` and internal handlers operate on a generic `CommandEnvelope<C>`.

    ```rust
    pub struct CommandEnvelope<C: Clone> {
        pub cmd: C,                         // Typed command payload
        pub auth: Option<AuthPayload>,      // Auth payload (shared with TransportEnvelope)
        pub meta: CommandMeta,              // Command metadata including `id`
    }
    ```

- The generic `C` allows internal command handling to be strongly typed, moving from string-based command labels and arbitrary JSON args to specific Rust enums (e.g., `BuiltInCommand`).

### **3. Conversion Between Formats**

- A dedicated conversion step is required to transform the wire-format `TransportEnvelope` into the internal `CommandEnvelope<C>`. This conversion involves:
  - Mapping `TransportEnvelope.id` to `CommandEnvelope.meta.id`.
  - Parsing `TransportEnvelope.cmd` (String) and `TransportEnvelope.args` (JsonValue) into the specific `C` type (e.g., `BuiltInCommand`).
  - Directly mapping `TransportEnvelope.auth` to `CommandEnvelope.auth`.
- **Conversion Rules**:
  - `TransportEnvelope::try_into_command_envelope<C: CommandTrait>(self, converter: Fn(String, JsonValue) -> Result<C, E>) -> Result<CommandEnvelope<C>, E>`
  - `CommandEnvelope<C>::into_transport_envelope(self) -> TransportEnvelope` (Straightforward mapping back).

### **4. `Transport` Trait Definition and Responsibilities**

- The `Transport` trait handles the wire-format conversion. Its methods are specifically scoped to `TransportEnvelope`.

    ```rust
    pub trait Transport: Send + Sync {
        type Error: std::error::Error + Send + Sync + 'static;

        /// Decodes raw bytes from the wire into a canonical `TransportEnvelope`.
        fn decode(&self, raw: &[u8]) -> Result<TransportEnvelope, Self::Error>;

        /// Encodes a `CommandResult` into raw bytes suitable for the wire, matching the context of the originating `TransportEnvelope`.
        /// Note: The `CommandContext` provided contains metadata derived during CommandEnvelope conversion.
        fn encode(&self, ctx: &CommandContext, result: &CommandResult) -> Result<Vec<u8>, Self::Error>;

        /// Maps transport-specific errors to a generic string.
        fn map_error(err: &Self::Error) -> String;
    }
    ```

- The `encode` method takes `CommandContext` and `CommandResult` because the wire format for *responses* is transport-specific and might depend on `CommandContext` metadata (e.g., `response_channel`).

### **5. Full Command Pipeline**

The lifecycle of a command request:

`Wire Bytes --(Transport::decode)--> TransportEnvelope --(Converter)--> CommandEnvelope<C> --(CommandRouter)--> CommandResult --(Transport::encode)--> Wire Bytes`

- **`Transport::decode`**: Converts raw wire bytes (e.g., HTTP request body, JSONL line) into a canonical `TransportEnvelope`. Errors in this stage are `Transport::Error`.
- **Converter**: A user-supplied function (`Fn(TransportEnvelope) -> Result<(CommandEnvelope<C>, CommandContext), String>`) maps `TransportEnvelope` to `CommandEnvelope<C>`. This involves parsing the generic `cmd` string and `args` `JsonValue` into the concrete type `C`. Errors in this stage typically map to `CommandError::Handler` or `CommandError::Auth` if parsing relies on identity.
- **`CommandRouter`**: Dispatches the `CommandEnvelope<C>` to the appropriate `CommandHandler`.
- **`Transport::encode`**: Converts the `CommandResult` and `CommandContext` back into raw wire bytes for the response. Errors in this stage are `Transport::Error`.

This ensures a clear, well-defined data flow with distinct responsibilities for each component.

## Rationale

- Keeps control-plane core decoupled from wire protocols.
- Single envelope simplifies testing and fuzzing (one schema).
- Trait allows pluggable transports and consistent error mapping.

## Consequences

- All transports must round-trip through `TransportEnvelope`.
- Additional transports (HTTP, gRPC, JSONL) implement `Transport` and wire conversion.

## Alternatives Considered

- Separate envelope per transport: rejected—adds divergence and duplicative code.
- Serde-tagged enum for multiple envelopes: unnecessary; single shape suffices.

## Notes

- JSON canonical form: `{"id": "...", "cmd": "...", "args": {...}, "auth": {...?}}`.
- Backward compatibility: new transport API lives in `control::transport`; existing router API unchanged.
