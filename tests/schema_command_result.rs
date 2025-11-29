//! Schema contract test for CommandResult against JSON Schema.
use jsonschema::JSONSchema;
use serde_json::json;

#[test]
fn command_result_matches_schema_variants() {
    let raw_schema = include_str!("../schemas/command-result.schema.json");
    let schema_val: serde_json::Value = serde_json::from_str(raw_schema).unwrap();
    let compiled = JSONSchema::compile(&schema_val).unwrap();

    let samples = [
        json!({"result": "ack"}),
        json!({"result": "value", "value": "ok"}),
        json!({"result": "list", "items": ["a", "b"]}),
        json!({"result": "reset"}),
        json!({"result": "error", "message": "boom", "kind": {"kind":"invalid_args","msg":"boom"}}),
    ];

    for sample in samples {
        assert_valid(&compiled, sample);
    }

    let invalid_samples = [
        json!({"result": "ack", "extra": true}), // extra field
        json!({"result": "value"}),              // missing value
        json!({"result": "error"}),              // missing fields
        json!({"result": "unknown"}),            // unknown result variant
        json!({}),                               // empty object
    ];

    for sample in invalid_samples {
        assert!(
            compiled.validate(&sample).is_err(),
            "schema unexpectedly accepted invalid sample: {sample}"
        );
    }
}
fn assert_valid(schema: &jsonschema::JSONSchema, value: serde_json::Value) {
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
