#![allow(missing_docs)]

use ninelives::control::{
    AuthMode, AuthPayload, AuthRegistry, BuiltInCommand, CommandEnvelope, CommandMeta,
    CommandResult, InMemoryHistory, PassthroughAuth,
};
use ninelives::ChannelTransport;
use std::sync::Arc;

fn env(cmd: BuiltInCommand) -> CommandEnvelope<BuiltInCommand> {
    CommandEnvelope {
        cmd,
        auth: Some(AuthPayload::Opaque(vec![])),
        meta: CommandMeta { id: "chan-1".into(), correlation_id: None, timestamp_millis: None },
    }
}

#[tokio::test]
async fn channel_transport_roundtrip() {
    let mut auth = AuthRegistry::new(AuthMode::First);
    auth.register(Arc::new(PassthroughAuth));
    let handler = Arc::new(ninelives::control::BuiltInHandler::default());
    let history: Arc<dyn ninelives::control::CommandHistory> = Arc::new(InMemoryHistory::default());
    let router = Arc::new(ninelives::control::CommandRouter::new(auth, handler, history));

    let transport = ChannelTransport::new(router);
    let res = transport.send(env(BuiltInCommand::List)).await.unwrap();
    assert_eq!(res, CommandResult::List(vec![]));
}
