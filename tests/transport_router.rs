#![allow(missing_docs)]

use ninelives::adaptive::Adaptive;
use ninelives::control::{
    AuthMode, AuthRegistry, BuiltInCommand, CommandEnvelope, CommandMeta, CommandResult,
    DefaultConfigRegistry, InMemoryHistory, PassthroughAuth,
};
use ninelives::{Transport, TransportEnvelope, TransportRouter};
use serde_json::json;
use std::sync::Arc;

// Simple JSON transport for testing.
#[derive(Clone)]
struct JsonTransport;

impl Transport for JsonTransport {
    type Error = serde_json::Error;

    fn decode(&self, raw: &[u8]) -> Result<TransportEnvelope, Self::Error> {
        serde_json::from_slice(raw)
    }

    fn encode(
        &self,
        ctx: &ninelives::control::CommandContext,
        result: &CommandResult,
    ) -> Result<Vec<u8>, Self::Error> {
        let out = match result {
            CommandResult::Ack => json!({"result":"ack","id":ctx.id}),
            CommandResult::Value(v) => json!({"result":"value","value":v,"id":ctx.id}),
            CommandResult::List(vs) => json!({"result":"list","items":vs,"id":ctx.id}),
            CommandResult::Reset => json!({"result":"reset","id":ctx.id}),
            CommandResult::Error(e) => {
                let mut v = serde_json::to_value(e).unwrap_or(json!({}));
                if let Some(obj) = v.as_object_mut() {
                    obj.insert("result".into(), "error".into());
                    obj.insert("message".into(), e.to_string().into());
                    obj.insert("id".into(), ctx.id.clone().into());
                }
                v
            }
        };
        serde_json::to_vec(&out)
    }

    fn map_error(err: &Self::Error) -> String {
        err.to_string()
    }
}

fn env_to_command(
    env: TransportEnvelope,
) -> Result<(CommandEnvelope<BuiltInCommand>, ninelives::control::CommandContext), String> {
    let cmd_lower = env.cmd.to_ascii_lowercase();
    let cmd = match cmd_lower.as_str() {
        "list" => BuiltInCommand::List,
        "set" => {
            let key = env.args.get("key").and_then(|v| v.as_str()).ok_or("missing key")?;
            let value = env.args.get("value").and_then(|v| v.as_str()).ok_or("missing value")?;
            BuiltInCommand::Set { key: key.into(), value: value.into() }
        }
        "get" => {
            let key = env.args.get("key").and_then(|v| v.as_str()).ok_or("missing key")?;
            BuiltInCommand::Get { key: key.into() }
        }
        "reset" => BuiltInCommand::Reset,
        "write_config" => {
            let path = env.args.get("path").and_then(|v| v.as_str()).ok_or("missing path")?;
            let value = env.args.get("value").and_then(|v| v.as_str()).ok_or("missing value")?;
            BuiltInCommand::WriteConfig { path: path.into(), value: value.into() }
        }
        "read_config" => {
            let path = env.args.get("path").and_then(|v| v.as_str()).ok_or("missing path")?;
            BuiltInCommand::ReadConfig { path: path.into() }
        }
        "list_config" => BuiltInCommand::ListConfig,
        _ => return Err(format!("unsupported cmd: {}", env.cmd)),
    };
    let ctx = ninelives::control::CommandContext {
        id: env.id,
        args: env.args,
        identity: None,
        response_channel: None,
    };
    let envelope = CommandEnvelope {
        cmd,
        auth: env.auth,
        meta: CommandMeta {
            id: ctx.id.clone(),
            correlation_id: Some(ctx.id.clone()),
            timestamp_millis: Some(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis(),
            ),
        },
    };
    Ok((envelope, ctx))
}

#[tokio::test]
async fn transport_router_roundtrip_list() {
    let mut auth = AuthRegistry::new(AuthMode::First);
    auth.register(Arc::new(PassthroughAuth));
    let history = Arc::new(InMemoryHistory::default());
    let handler = Arc::new(ninelives::control::BuiltInHandler::default());
    let router = ninelives::control::CommandRouter::new(auth, handler, history);

    let t_router = TransportRouter::new(router, JsonTransport, env_to_command);

    let raw = json!({
        "id": "cmd-99",
        "cmd": "list",
        "args": {},
        "auth": null
    })
    .to_string();

    let bytes = t_router.handle(raw.as_bytes()).await.unwrap();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(v["result"], "list");
    assert_eq!(v["id"], "cmd-99");
}

#[tokio::test]
async fn transport_router_set_get_reset() {
    let mut auth = AuthRegistry::new(AuthMode::First);
    auth.register(Arc::new(PassthroughAuth));
    let history = Arc::new(InMemoryHistory::default());
    let handler = Arc::new(ninelives::control::BuiltInHandler::default());
    let router = ninelives::control::CommandRouter::new(auth, handler, history);
    let t_router = TransportRouter::new(router, JsonTransport, env_to_command);

    // Set
    let raw_set = json!({
        "id": "cmd-set",
        "cmd": "SET",
        "args": { "key": "k", "value": "v" },
        "auth": null
    })
    .to_string();
    let bytes = t_router.handle(raw_set.as_bytes()).await.unwrap();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(v["result"], "ack");

    // Get
    let raw_get = json!({
        "id": "cmd-get",
        "cmd": "get",
        "args": { "key": "k" },
        "auth": null
    })
    .to_string();
    let bytes = t_router.handle(raw_get.as_bytes()).await.unwrap();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(v["result"], "value");
    assert_eq!(v["value"], "v");
    assert_eq!(v["id"], "cmd-get");

    // Reset then Get returns default empty string
    let raw_reset = json!({
        "id": "cmd-reset",
        "cmd": "reset",
        "args": {},
        "auth": null
    })
    .to_string();
    let _ = t_router.handle(raw_reset.as_bytes()).await.unwrap();

    let bytes = t_router.handle(raw_get.as_bytes()).await.unwrap();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(v["result"], "value");
    assert_eq!(v["value"], "");
}

#[tokio::test]
async fn transport_router_get_unknown_errors() {
    let mut auth = AuthRegistry::new(AuthMode::First);
    auth.register(Arc::new(PassthroughAuth));
    let history = Arc::new(InMemoryHistory::default());
    let handler = Arc::new(ninelives::control::BuiltInHandler::default());
    let router = ninelives::control::CommandRouter::new(auth, handler, history);
    let t_router = TransportRouter::new(router, JsonTransport, env_to_command);

    let raw_get = json!({
        "id": "cmd-get-missing",
        "cmd": "GET",
        "args": { "key": "missing" },
        "auth": null
    })
    .to_string();
    let bytes = t_router.handle(raw_get.as_bytes()).await.unwrap();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(v["result"], "value");
    assert_eq!(v["value"], "");
}

#[tokio::test]
async fn transport_router_write_and_read_config() {
    let mut auth = AuthRegistry::new(AuthMode::First);
    auth.register(Arc::new(PassthroughAuth));
    let mut cfg = DefaultConfigRegistry::new();
    cfg.register_fromstr("retry.max_attempts", Adaptive::new(3usize));
    let history = Arc::new(InMemoryHistory::default());
    let handler = Arc::new(ninelives::control::BuiltInHandler::default().with_config_registry(cfg));
    let router = ninelives::control::CommandRouter::new(auth, handler, history);
    let t_router = TransportRouter::new(router, JsonTransport, env_to_command);

    let raw_write = json!({
        "id": "cmd-write",
        "cmd": "write_config",
        "args": { "path": "retry.max_attempts", "value": "5" },
        "auth": null
    })
    .to_string();
    let bytes = t_router.handle(raw_write.as_bytes()).await.unwrap();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(v["result"], "ack");

    let raw_read = json!({
        "id": "cmd-read",
        "cmd": "read_config",
        "args": { "path": "retry.max_attempts" },
        "auth": null
    })
    .to_string();
    let bytes = t_router.handle(raw_read.as_bytes()).await.unwrap();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(v["result"], "value");
    assert_eq!(v["value"], "5");
}

#[tokio::test]
async fn transport_router_write_config_errors_without_registry() {
    let mut auth = AuthRegistry::new(AuthMode::First);
    auth.register(Arc::new(PassthroughAuth));
    let history = Arc::new(InMemoryHistory::default());
    let handler = Arc::new(ninelives::control::BuiltInHandler::default());
    let router = ninelives::control::CommandRouter::new(auth, handler, history);
    let t_router = TransportRouter::new(router, JsonTransport, env_to_command);

    let raw_write = json!({
        "id": "cmd-write-err",
        "cmd": "write_config",
        "args": { "path": "retry.max_attempts", "value": "5" },
        "auth": null
    })
    .to_string();
    let err = t_router.handle(raw_write.as_bytes()).await.unwrap_err();
    assert!(err.contains("config registry missing"), "expected missing registry error, got {err}");
}

#[tokio::test]
async fn transport_router_malformed_args_error() {
    let mut auth = AuthRegistry::new(AuthMode::First);
    auth.register(Arc::new(PassthroughAuth));
    let history = Arc::new(InMemoryHistory::default());
    let handler = Arc::new(ninelives::control::BuiltInHandler::default());
    let router = ninelives::control::CommandRouter::new(auth, handler, history);
    let t_router = TransportRouter::new(router, JsonTransport, env_to_command);

    let raw_set = json!({
        "id": "cmd-bad",
        "cmd": "set",
        "args": { "value": "missing-key" },
        "auth": null
    })
    .to_string();
    let err = t_router.handle(raw_set.as_bytes()).await.unwrap_err();
    assert!(err.contains("missing key"));
}

#[tokio::test]
async fn transport_router_rejects_malformed_schema() {
    let mut auth = AuthRegistry::new(AuthMode::First);
    auth.register(Arc::new(PassthroughAuth));
    let history = Arc::new(InMemoryHistory::default());
    let handler = Arc::new(ninelives::control::BuiltInHandler::default());
    let router = ninelives::control::CommandRouter::new(auth, handler, history);
    let t_router = TransportRouter::new(router, JsonTransport, env_to_command);

    // Opaque auth requires minItems: 1 in schema, but Vec<u8> allows empty.
    // This verifies that schema validation runs after decode.
    let raw_bad = json!({
        "id": "cmd-schema",
        "cmd": "list",
        "args": {},
        "auth": { "Opaque": [] }
    })
    .to_string();

    #[cfg(feature = "schema-validation")]
    {
        let err = t_router.handle(raw_bad.as_bytes()).await.unwrap_err();
        // jsonschema error message typically mentions the constraint
        assert!(
            err.contains("minItems") || err.contains("less than"),
            "expected schema validation error (minItems), got: {err}"
        );
    }
    #[cfg(not(feature = "schema-validation"))]
    {
        // Without schema validation, the payload passes; ensure it still routes and returns list.
        let bytes = t_router.handle(raw_bad.as_bytes()).await.unwrap();
        let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(v["result"], "list");
    }
}
