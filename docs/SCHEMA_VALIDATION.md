# Schema Validation

Schema validation ensures control-plane envelopes and results conform to the documented JSON schemas.

## Runtime toggle
- Enabled by default.
- Controlled via environment variable `NINELIVES_SCHEMA_VALIDATION`.
  - Disable with any of: `0`, `false`, `False`, `FALSE`.
  - Any other value (or unset) keeps validation **on**.

## What is validated
- Incoming `TransportEnvelope` (id, cmd, args, auth) before routing.
- Outgoing `CommandResult` after handler execution.

## Where it runs
- Validation is performed inside `TransportRouter::handle`, and when layered, by `SchemaValidationLayer`/`SchemaValidated` for supported transports.

## Failure behavior
- Invalid envelope/result returns an error before executing the command (for envelopes) or before encoding (for results).

## Compile-time notes
- There is **no** Cargo feature named `schema-validation`. Validation code ships with the `control` feature; enable or disable at runtime via `NINELIVES_SCHEMA_VALIDATION`.

## Examples

```bash
# Default (on)
cargo run -p ninelives-cookbook --example control_plane

# Disable validation for debugging
NINELIVES_SCHEMA_VALIDATION=0 cargo run -p ninelives-cookbook --example control_plane
```

```rust
// TransportRouter will validate by default; you can still disable at runtime:
std::env::set_var("NINELIVES_SCHEMA_VALIDATION", "0");
```
