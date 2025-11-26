use ninelives::control::{
    AuthMode, AuthPayload, AuthRegistry, CommandEnvelope, CommandMeta, CommandResult, ConfigRegistry, InMemoryHistory,
};
use ninelives::control::{BuiltInCommand, BuiltInHandler, CommandRouter, PassthroughAuth};
use ninelives::{Backoff, Jitter, RetryPolicy};
use std::time::Duration;

#[tokio::test]
async fn config_commands_update_retry_adaptive() {
    let policy = RetryPolicy::<TestError>::builder()
        .max_attempts(1)
        .backoff(Backoff::constant(Duration::from_millis(1)))
        .with_jitter(Jitter::None)
        .build()
        .unwrap();

    let adapt = policy.adaptive_max_attempts();

    let mut registry = ConfigRegistry::new();
    registry.register_fromstr("max_attempts", adapt.clone());

    let handler = BuiltInHandler::default().with_config_registry(registry);
    let mut auth = AuthRegistry::new(AuthMode::First);
    auth.register(std::sync::Arc::new(PassthroughAuth));
    let history = std::sync::Arc::new(InMemoryHistory::default());
    let router = CommandRouter::new(auth, std::sync::Arc::new(handler), history);

    let env = CommandEnvelope {
        cmd: BuiltInCommand::WriteConfig { path: "max_attempts".into(), value: "3".into() },
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
