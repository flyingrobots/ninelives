#![allow(missing_docs)]

use ninelives::adaptive::Adaptive;
use ninelives::control::{
    AuthMode, AuthRegistry, CommandEnvelope, CommandMeta, CommandResult,
    DefaultConfigRegistry, InMemoryHistory, PassthroughAuth, BuiltInCommandFactory,
};
use ninelives::control::command::CommandFactory;
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
            _ => json!({"result":"unknown","id":ctx.id}),
        };
        serde_json::to_vec(&out)
    }

    fn map_error(err: &Self::Error) -> String {
        err.to_string()
    }
}

fn env_to_command(
    env: TransportEnvelope,
) -> Result<(CommandEnvelope, ninelives::control::CommandContext), String> {
    let factory = BuiltInCommandFactory;
    let cmd = factory.create(&env.cmd, &env.args)?;

    let ctx = ninelives::control::CommandContext {
        id: env.id.clone(),
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
    let handler = Arc::new(ninelives::control::BuiltInHandler::default());
    let history: Arc<dyn ninelives::control::CommandHistory> = Arc::new(InMemoryHistory::default());
    let router = ninelives::control::CommandRouter::new(auth, handler, history);

    let t_router = TransportRouter::new(router, JsonTransport, env_to_command);

    let raw = r#"{"id":"list-1","cmd":"list","args":{}}"#.as_bytes();
    let out = t_router.handle(raw).await.unwrap();
    let v: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(v["result"], "list");
    assert_eq!(v["items"], json!([]));
}

#[tokio::test]
async fn transport_router_set_get() {
    let mut auth = AuthRegistry::new(AuthMode::First);
    auth.register(Arc::new(PassthroughAuth));
    let handler = Arc::new(ninelives::control::BuiltInHandler::default());
    let history: Arc<dyn ninelives::control::CommandHistory> = Arc::new(InMemoryHistory::default());
    let router = ninelives::control::CommandRouter::new(auth, handler, history);

    let t_router = TransportRouter::new(router, JsonTransport, env_to_command);

    let set_raw = r#"{"id":"set-1","cmd":"set","args":{"key":"k1","value":"v1"}}"#.as_bytes();
    let set_out = t_router.handle(set_raw).await.unwrap();
    let set_v: serde_json::Value = serde_json::from_slice(&set_out).unwrap();
    assert_eq!(set_v["result"], "ack");

    let get_raw = r#"{"id":"get-1","cmd":"get","args":{"key":"k1"}}"#.as_bytes();
    let get_out = t_router.handle(get_raw).await.unwrap();
    let get_v: serde_json::Value = serde_json::from_slice(&get_out).unwrap();
    assert_eq!(get_v["result"], "value");
    assert_eq!(get_v["value"], "v1");
}

#[tokio::test]
async fn transport_router_write_config_with_registry() {
    let registry = DefaultConfigRegistry::new();
    registry.register_fromstr("max_attempts", Adaptive::new(1usize));

    let handler = ninelives::control::BuiltInHandler::default().with_config_registry(registry);

    let mut auth = AuthRegistry::new(AuthMode::First);
    auth.register(Arc::new(PassthroughAuth));
    let history: Arc<dyn ninelives::control::CommandHistory> = Arc::new(InMemoryHistory::default());
    let router = ninelives::control::CommandRouter::new(auth, Arc::new(handler), history);

    let t_router = TransportRouter::new(router, JsonTransport, env_to_command);

    let raw =
        r#"{"id":"write-cfg-1","cmd":"write_config","args":{"path":"max_attempts","value":"5"}}"#
            .as_bytes();
    let out = t_router.handle(raw).await.unwrap();
    let v: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(v["result"], "ack");
}

#[tokio::test]
async fn transport_router_size_limit() {
    let mut auth = AuthRegistry::new(AuthMode::First);
    auth.register(Arc::new(PassthroughAuth));
    let handler = Arc::new(ninelives::control::BuiltInHandler::default());
    let history: Arc<dyn ninelives::control::CommandHistory> = Arc::new(InMemoryHistory::default());
    let router = ninelives::control::CommandRouter::new(auth, handler, history);

    let t_router = TransportRouter::new(router, JsonTransport, env_to_command);

    let big = vec![0u8; 2 * 1024 * 1024]; // 2 MiB
    let err = t_router.handle(&big).await.unwrap_err();
    assert!(err.contains("maximum size"));
}

#[tokio::test]
async fn transport_router_invalid_json() {
    let mut auth = AuthRegistry::new(AuthMode::First);
    auth.register(Arc::new(PassthroughAuth));
    let handler = Arc::new(ninelives::control::BuiltInHandler::default());
    let history: Arc<dyn ninelives::control::CommandHistory> = Arc::new(InMemoryHistory::default());
    let router = ninelives::control::CommandRouter::new(auth, handler, history);

    let t_router = TransportRouter::new(router, JsonTransport, env_to_command);

    let bad = b"not json at all";
    let err = t_router.handle(bad).await.unwrap_err();
    assert!(err.contains("expected") || err.contains("EOF"));
}

#[tokio::test]
async fn transport_router_bad_command() {
    let mut auth = AuthRegistry::new(AuthMode::First);
    auth.register(Arc::new(PassthroughAuth));
    let handler = Arc::new(ninelives::control::BuiltInHandler::default());
    let history: Arc<dyn ninelives::control::CommandHistory> = Arc::new(InMemoryHistory::default());
    let router = ninelives::control::CommandRouter::new(auth, handler, history);

    let t_router = TransportRouter::new(router, JsonTransport, env_to_command);

    let bad = r#"{"id":"bad-1","cmd":"bad_command_xyz","args":{}}"#.as_bytes();
    let err = t_router.handle(bad).await.unwrap_err();
    assert!(err.contains("unknown"));
}

#[tokio::test]
async fn transport_router_clone_and_call() {
    let mut auth = AuthRegistry::new(AuthMode::First);
    auth.register(Arc::new(PassthroughAuth));
    let handler = Arc::new(ninelives::control::BuiltInHandler::default());
    let history: Arc<dyn ninelives::control::CommandHistory> = Arc::new(InMemoryHistory::default());
    let router = ninelives::control::CommandRouter::new(auth, handler, history);

    let t_router = TransportRouter::new(router, JsonTransport, env_to_command);
    let t_router_clone = t_router.clone();

    let raw = r#"{"id":"list-clone","cmd":"list","args":{}}"#.as_bytes();
    let out = t_router_clone.handle(raw).await.unwrap();
    let v: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(v["result"], "list");
}
