//! Schema contract test for TransportEnvelope against JSON Schema.
use jsonschema::JSONSchema;
use serde_json::json;

#[test]
fn transport_envelope_matches_schema() {
    let raw_schema = include_str!("../schemas/transport-envelope.schema.json");
    let schema_val: serde_json::Value = serde_json::from_str(raw_schema).unwrap();
    let compiled = JSONSchema::compile(&schema_val).unwrap();

    let envelope = json!({
        "id": "cmd-123",
        "cmd": "write_config",
        "args": {"path": "retry.max_attempts", "value": "5"},
        "auth": {"type": "opaque", "data": "deadbeef"}
    });

    assert_valid(&compiled, envelope);
}
fn assert_valid(schema: &jsonschema::JSONSchema, value: serde_json::Value) {
    if let Err(errs) = schema.validate(&value) {
        let msg = errs.map(|e| e.to_string()).collect::<Vec<_>>().join(", ");
        panic!("{}", msg);
    }
}
