use ninelives::control::{AuthMode, AuthPayload, AuthRegistry, CommandEnvelope, CommandMeta, CommandResult};
use ninelives::control::{BuiltInCommand, BuiltInHandler, CommandRouter, PassthroughAuth};
use ninelives::retry::RetryPolicy;
use ninelives::backoff::Backoff;
use ninelives::jitter::Jitter;
use std::time::Duration;
use tower::ServiceBuilder;

#[tokio::test]
async fn config_commands_update_retry_adaptive() {
    let policy = RetryPolicy::<TestError>::builder()
        .max_attempts(1)
        .backoff(Backoff::constant(Duration::from_millis(1)))
        .with_jitter(Jitter::None)
        .build()
        .unwrap();

    let adapt = policy.adaptive_max_attempts();
    let handler = BuiltInHandler::default();
    let mut auth = AuthRegistry::new(AuthMode::First);
    auth.register(std::sync::Arc::new(PassthroughAuth));
    let history = std::sync::Arc::new(super::InMemoryHistory::default());
    let router = CommandRouter::new(auth, std::sync::Arc::new(handler), history);

    // Simulate WriteConfig by using BuiltInCommand::Set on a key
    let env = CommandEnvelope {
        cmd: BuiltInCommand::Set { key: "max_attempts".into(), value: "3".into() },
        auth: Some(AuthPayload::Opaque(vec![])),
        meta: CommandMeta { id: "1".into(), correlation_id: None, timestamp_millis: None },
    };
    let res = router.execute(env).await.unwrap();
    assert_eq!(res, CommandResult::Ack);
    assert_eq!(*adapt.get(), 3);
}

#[derive(Debug, Clone, thiserror::Error)]
#[error("test")] 
struct TestError;
