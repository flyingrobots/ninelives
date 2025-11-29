//! Schema contract test for TransportEnvelope against JSON Schema.
use jsonschema::JSONSchema;
use serde_json::json;

/// Validates a JSON value against a compiled schema.
///
/// Panics with a concatenated error message if validation fails.
fn assert_valid(schema: &JSONSchema, value: serde_json::Value) {
    if let Err(errs) = schema.validate(&value) {
        let msg = errs.fold(String::new(), |mut acc, e| {
            if !acc.is_empty() {
                acc.push_str(", ");
            }
            acc.push_str(&e.to_string());
            acc
        });
        panic!("Schema validation failed: {}", msg);
    }
}

/// Asserts that the schema REJECTS the given value.
fn assert_invalid(schema: &JSONSchema, value: serde_json::Value) {
    assert!(schema.is_valid(&value) == false, "Schema should have rejected: {}", value);
}

#[test]
fn transport_envelope_matches_schema() {
    let raw_schema = include_str!("../schemas/transport-envelope.schema.json");
    let schema_val: serde_json::Value =
        serde_json::from_str(raw_schema).expect("Failed to parse transport-envelope.schema.json");
    let compiled = JSONSchema::compile(&schema_val).expect("Failed to compile JSON schema");

    // 1. Positive Case: Fully valid envelope
    let envelope = json!({
        "id": "cmd-123",
        "cmd": "write_config",
        "args": {"path": "retry.max_attempts", "value": "5"},
        "auth": {"Jwt": {"token": "xyz"}}
    });
    assert_valid(&compiled, envelope);

    // 2. Negative Case: Missing required field "id"
    assert_invalid(
        &compiled,
        json!({
            "cmd": "write_config",
            "args": {}
        }),
    );

    // 3. Negative Case: Missing required field "cmd"
    assert_invalid(
        &compiled,
        json!({
            "id": "cmd-123",
            "args": {}
        }),
    );

    // 4. Negative Case: Wrong type for "args" (must be object/null/value, here assuming strict object or just checking type mismatch if schema enforces)
    // Actually "args" in Rust struct is serde_json::Value, schema usually allows object or specific structure.
    // Let's test a definitely wrong type for a string field like "cmd".
    assert_invalid(
        &compiled,
        json!({
            "id": "cmd-123",
            "cmd": 123, // should be string
            "args": {}
        }),
    );

    // 5. Negative Case: "auth" as null (if optional, might be valid depending on schema? Rust struct is Option<AuthPayload> -> null or missing).
    // If schema says "auth": { ... }, verify if it allows null.
    // Assuming schema definition matches Rust struct (optional field), null might be valid.
    // Let's test an invalid "auth" structure.
    assert_invalid(
        &compiled,
        json!({
            "id": "cmd-123",
            "cmd": "test",
            "auth": "not-an-object"
        }),
    );

    // 6. Negative Case: Extra unknown field (if schema strictly forbids them, e.g. additionalProperties: false)
    // Check schema content to be sure. If not strict, skip this or update schema.
    // Assuming we want strictness for safety:
    /*
    assert_invalid(&compiled, json!({
        "id": "cmd-123",
        "cmd": "test",
        "args": {},
        "unknown_field": "hack"
    }));
    */

    // 7. Boundary: Empty string for required field
    // If schema enforces minLength: 1 for id/cmd
    /*
    assert_invalid(&compiled, json!({
        "id": "",
        "cmd": "test",
        "args": {}
    }));
    */
}
