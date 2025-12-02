#![allow(missing_docs)]

use ninelives::control::{
    AuthMode, AuthPayload, AuthRegistry, CommandEnvelope, CommandMeta,
    CommandResult, PassthroughAuth, ListCommand,
};
use ninelives::AuthorizationLayer;
use std::future::Ready;
use std::task::{Context, Poll};
use tower::{Layer, Service, ServiceExt};

#[derive(Clone)]
struct Echo;

impl Service<CommandEnvelope> for Echo {
    type Response = CommandResult;
    type Error = ninelives::control::CommandError;
    type Future = Ready<Result<CommandResult, Self::Error>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, _req: CommandEnvelope) -> Self::Future {
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
        cmd: Box::new(ListCommand),
        auth: Some(AuthPayload::Opaque(vec![])),
        meta: CommandMeta { id: "t".into(), correlation_id: None, timestamp_millis: None },
    };
    let out = svc.oneshot(env).await.unwrap();
    assert_eq!(out, CommandResult::Ack);
}
