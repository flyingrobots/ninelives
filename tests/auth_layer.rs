#![allow(missing_docs)]

mod common;

use ninelives::control::{
    AuthContext, AuthError, AuthMode, AuthPayload, AuthProvider, AuthRegistry, BuiltInCommand,
    CommandEnvelope, CommandMeta, CommandResult, PassthroughAuth,
};
use ninelives::control::router::{DEFAULT_HISTORY_CAPACITY, MemoryAuditSink};
use ninelives::AuthorizationLayer;
use std::sync::Arc;
use tower::{Layer, Service, ServiceExt};

use common::test_helpers;

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
    let now =
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis();
    test_helpers::create_test_envelope(cmd, Some("cmd-1"), Some("corr-1"), None, Some(now))
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

// ...
fn make_router(
    auth: Arc<dyn AuthProvider>,
    audit: Arc<MemoryAuditSink>,
) -> ninelives::control::CommandRouter<BuiltInCommand> {
    let mut reg = AuthRegistry::new(AuthMode::First);
    reg.register(auth);
    let history: Arc<dyn ninelives::control::CommandHistory> =
        Arc::new(ninelives::control::InMemoryHistory::default());
    let handler = Arc::new(ninelives::control::BuiltInHandler::default());
    ninelives::control::CommandRouter::new(reg, handler, history).with_audit(audit)
}

#[tokio::test]
async fn command_router_audits_denial() {
    let audit = Arc::new(MemoryAuditSink::new(DEFAULT_HISTORY_CAPACITY));
    let router = make_router(Arc::new(DenyAuth), audit.clone());

    let res = router.execute(env(BuiltInCommand::List)).await;
    assert!(matches!(res, Err(ninelives::control::CommandError::Auth(_))));

    let records = audit.records().await;
    assert_eq!(records.len(), 1);
    let status = &records[0].status;
    assert!(
        status.contains("denied") && status.contains("unauthenticated"),
        "unexpected status: {}",
        status
    );
}

#[tokio::test]
async fn command_router_audits_success() {
    let audit = Arc::new(MemoryAuditSink::new(DEFAULT_HISTORY_CAPACITY));
    let router = make_router(Arc::new(PassthroughAuth), audit.clone());

    let res = router.execute(env(BuiltInCommand::List)).await.unwrap();
    assert_eq!(res, CommandResult::List(vec![]));

    let records = audit.records().await;
    assert_eq!(records.len(), 1);
    let status = &records[0].status;
    assert!(status == "ok" || status.contains("ok"), "unexpected status: {}", status);
}
