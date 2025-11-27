#![allow(missing_docs)]

use ninelives::control::CommandContext;
use serde_json::json;

#[test]
fn command_context_roundtrip() {
    let input = r#"{"id":"cmd-1","args":{"foo":1},"identity":"alice","response_channel":"ch-1"}"#;
    let ctx: CommandContext = serde_json::from_str(input).expect("deser");
    assert_eq!(ctx.id, "cmd-1");
    assert_eq!(ctx.identity.as_deref(), Some("alice"));
    assert_eq!(ctx.response_channel.as_deref(), Some("ch-1"));
    assert_eq!(ctx.args, json!({"foo":1}));

    let out = serde_json::to_string(&ctx).expect("ser");
    let ctx2: CommandContext = serde_json::from_str(&out).expect("deser");
    assert_eq!(ctx, ctx2);
}

#[test]
fn command_context_defaults_args() {
    let input = r#"{"id":"cmd-2"}"#;
    let ctx: CommandContext = serde_json::from_str(input).expect("deser");
    assert_eq!(ctx.id, "cmd-2");
    assert_eq!(ctx.args, serde_json::Value::Null);
    assert!(ctx.identity.is_none());
}

#[test]
fn command_context_missing_id_fails() {
    let input = r#"{"args":{"foo":1}}"#;
    let err = serde_json::from_str::<CommandContext>(input).unwrap_err();
    assert!(err.is_data());
}
