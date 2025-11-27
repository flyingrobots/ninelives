#![allow(missing_docs)]

use ninelives::control::AuthPayload;
use ninelives::TransportEnvelope;
use serde_json::json;

#[test]
fn transport_envelope_roundtrip_json() {
    let env = TransportEnvelope {
        id: "cmd-1".into(),
        cmd: "write_config".into(),
        args: json!({"path": "max_attempts", "value": "3"}),
        auth: Some(AuthPayload::Opaque(vec![1, 2, 3])),
    };

    let serialized = serde_json::to_string(&env).unwrap();
    let de: TransportEnvelope = serde_json::from_str(&serialized).unwrap();
    assert_eq!(env, de);
}
