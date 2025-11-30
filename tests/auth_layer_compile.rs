#![allow(missing_docs)]

use ninelives::control::{
    AuthMode, AuthPayload, AuthRegistry, BuiltInCommand, CommandEnvelope, CommandMeta,
    CommandResult, PassthroughAuth,
};
use ninelives::AuthorizationLayer;
use std::future::Ready;
use std::task::{Context, Poll};
use tower::{Layer, Service, ServiceExt};

#[derive(Clone)]
struct Echo;

impl Service<CommandEnvelope<BuiltInCommand>> for Echo {
    type Response = CommandResult;
    type Error = ninelives::control::CommandError;
    type Future = Ready<Result<CommandResult, Self::Error>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, _req: CommandEnvelope<BuiltInCommand>) -> Self::Future {
        std::future::ready(Ok(CommandResult::Ack))
    }
}

#[tokio::test]
async fn authorization_layer_compiles_and_passes_through() {
    let mut registry = AuthRegistry::new(AuthMode::First);
    registry.register(std::sync::Arc::new(PassthroughAuth));

    let layer = AuthorizationLayer::new(registry);
    let svc = layer.layer(Echo);

    let env = CommandEnvelope {
        cmd: BuiltInCommand::List,
        auth: Some(AuthPayload::Opaque(vec![])),
        meta: CommandMeta { id: "t".into(), correlation_id: None, timestamp_millis: None },
    };
    let out = svc.oneshot(env).await.unwrap();
    assert_eq!(out, CommandResult::Ack);
}
