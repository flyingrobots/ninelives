#![allow(missing_docs)]

use ninelives::control::{
    AuthMode, AuthRegistry, BuiltInCommand, CommandEnvelope, CommandMeta, CommandResult,
    InMemoryHistory, PassthroughAuth,
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
            CommandResult::Error(e) => json!({"result":"error","message":e,"id":ctx.id}),
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
    // For test, only support BuiltInCommand::List
    let cmd = match env.cmd.as_str() {
        "list" | "List" => BuiltInCommand::List,
        other => return Err(format!("unsupported cmd: {other}")),
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
        meta: CommandMeta { id: ctx.id.clone(), correlation_id: None, timestamp_millis: None },
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
