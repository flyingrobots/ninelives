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
        json!({"result": "error", "message": "boom"}),
    ];

    for sample in samples {
        assert_valid(&compiled, sample);
    }
}
fn assert_valid(schema: &jsonschema::JSONSchema, value: serde_json::Value) {
    if let Err(errs) = schema.validate(&value) {
        let msg = errs.map(|e| e.to_string()).collect::<Vec<_>>().join(", ");
        panic!("{}", msg);
    }
}
