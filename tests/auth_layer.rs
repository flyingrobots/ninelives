#![allow(missing_docs)]

use ninelives::control::{
    AuthContext, AuthError, AuthMode, AuthPayload, AuthProvider, AuthRegistry, BuiltInCommand,
    CommandEnvelope, CommandMeta, CommandResult, PassthroughAuth,
};
use ninelives::AuthorizationLayer;
use std::sync::Arc;
use tower::{Layer, Service, ServiceExt};

#[derive(Clone)]
struct RecordingSvc {
    called: Arc<std::sync::atomic::AtomicBool>,
}

impl Service<CommandEnvelope<BuiltInCommand>> for RecordingSvc {
    type Response = CommandResult;
    type Error = ninelives::control::CommandError;
    type Future = std::future::Ready<Result<Self::Response, Self::Error>>;

    fn poll_ready(
        &mut self,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        std::task::Poll::Ready(Ok(()))
    }

    fn call(&mut self, _req: CommandEnvelope<BuiltInCommand>) -> Self::Future {
        self.called.store(true, std::sync::atomic::Ordering::SeqCst);
        std::future::ready(Ok(CommandResult::Ack))
    }
}

#[derive(Clone)]
struct DenyAuth;
impl AuthProvider for DenyAuth {
    fn name(&self) -> &'static str {
        "deny"
    }
    fn authenticate(
        &self,
        _meta: &CommandMeta,
        _auth: Option<&AuthPayload>,
    ) -> Result<AuthContext, AuthError> {
        Err(AuthError::Unauthenticated("denied".into()))
    }
}

fn env(cmd: BuiltInCommand) -> CommandEnvelope<BuiltInCommand> {
    CommandEnvelope {
        cmd,
        auth: Some(AuthPayload::Opaque(vec![])),
        meta: CommandMeta { id: "cmd-1".into(), correlation_id: None, timestamp_millis: None },
    }
}

#[tokio::test]
async fn authorization_layer_allows_and_forwards() {
    let mut reg = AuthRegistry::new(AuthMode::First);
    reg.register(Arc::new(PassthroughAuth));

    let layer = AuthorizationLayer::new(reg);
    let called = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let svc = RecordingSvc { called: called.clone() };
    let svc = layer.layer(svc);
    let res = svc.oneshot(env(BuiltInCommand::List)).await.unwrap();
    assert!(called.load(std::sync::atomic::Ordering::SeqCst));
    assert_eq!(res, CommandResult::Ack);
}

#[tokio::test]
async fn authorization_layer_denies_and_blocks() {
    let mut reg = AuthRegistry::new(AuthMode::First);
    reg.register(Arc::new(DenyAuth));

    let layer = AuthorizationLayer::new(reg);
    let called = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let svc = RecordingSvc { called: called.clone() };
    let svc = layer.layer(svc);
    let res = svc.oneshot(env(BuiltInCommand::List)).await;
    assert!(!called.load(std::sync::atomic::Ordering::SeqCst));
    assert!(matches!(res, Err(ninelives::control::CommandError::Auth(_))));
}
