#![allow(missing_docs)]

use ninelives::adaptive::Adaptive;
use ninelives::circuit_breaker_registry::CircuitBreakerRegistry;
use ninelives::control::{
    AuthMode, AuthPayload, AuthRegistry, CommandEnvelope, CommandMeta, CommandResult,
    ConfigRegistry, InMemoryHistory,
};
use ninelives::control::{BuiltInCommand, BuiltInHandler, CommandRouter, PassthroughAuth};
use ninelives::{Backoff, Jitter, RetryPolicy};
use ninelives::{CircuitBreakerConfig, CircuitBreakerLayer, CircuitState};
use std::future::Ready;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Duration;
use tower::{Layer, Service, ServiceExt};

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

#[tokio::test]
async fn list_config_returns_registered_keys() {
    let mut registry = ConfigRegistry::new();
    registry.register_fromstr("max_attempts", Adaptive::new(1usize));
    registry.register_fromstr("timeout_ms", Adaptive::new(100usize));

    let handler = BuiltInHandler::default().with_config_registry(registry);
    let mut auth = AuthRegistry::new(AuthMode::First);
    auth.register(std::sync::Arc::new(PassthroughAuth));
    let history = std::sync::Arc::new(InMemoryHistory::default());
    let router = CommandRouter::new(auth, std::sync::Arc::new(handler), history);

    let env = CommandEnvelope {
        cmd: BuiltInCommand::ListConfig,
        auth: Some(AuthPayload::Opaque(vec![])),
        meta: CommandMeta { id: "lc".into(), correlation_id: None, timestamp_millis: None },
    };
    let res = router.execute(env).await.unwrap();
    assert_eq!(res, CommandResult::List(vec!["max_attempts".into(), "timeout_ms".into()]));
}

#[tokio::test]
async fn get_state_reports_open_breaker() {
    let registry = CircuitBreakerRegistry::default();

    // Create breaker and force it to open.
    let cfg =
        CircuitBreakerConfig::new(1, Duration::from_millis(1), 1).unwrap().with_id("cb_state");
    let layer = CircuitBreakerLayer::new(cfg).unwrap().with_registry(registry.clone());
    let mut svc = layer.layer(FailingSvc);
    let _ = svc.ready().await.unwrap().call(()).await;

    let handler = BuiltInHandler::default().with_circuit_breaker_registry(registry);
    let mut auth = AuthRegistry::new(AuthMode::First);
    auth.register(std::sync::Arc::new(PassthroughAuth));
    let history = Arc::new(InMemoryHistory::default());
    let router = CommandRouter::new(auth, Arc::new(handler), history);

    let env = CommandEnvelope {
        cmd: BuiltInCommand::GetState,
        auth: Some(AuthPayload::Opaque(vec![])),
        meta: CommandMeta { id: "gs".into(), correlation_id: None, timestamp_millis: None },
    };
    let res = router.execute(env).await.unwrap();
    if let CommandResult::Value(s) = res {
        let v: serde_json::Value = serde_json::from_str(&s).unwrap();
        assert_eq!(v["breakers"]["cb_state"], "Open");
    } else {
        panic!("expected Value result");
    }
}

#[tokio::test]
async fn reset_circuit_breaker_command() {
    let registry = CircuitBreakerRegistry::default();
    registry.register_new("cb1".into());

    let handler = BuiltInHandler::default().with_circuit_breaker_registry(registry);
    let mut auth = AuthRegistry::new(AuthMode::First);
    auth.register(std::sync::Arc::new(PassthroughAuth));
    let history = Arc::new(InMemoryHistory::default());
    let router = CommandRouter::new(auth, Arc::new(handler), history);

    let env = CommandEnvelope {
        cmd: BuiltInCommand::ResetCircuitBreaker { id: "cb1".into() },
        auth: Some(AuthPayload::Opaque(vec![])),
        meta: CommandMeta { id: "2".into(), correlation_id: None, timestamp_millis: None },
    };
    let res = router.execute(env).await.unwrap();
    assert_eq!(res, CommandResult::Ack);
}

#[derive(Clone)]
struct FailingSvc;

impl Service<()> for FailingSvc {
    type Response = ();
    type Error = TestError;
    type Future = Ready<Result<(), TestError>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, _req: ()) -> Self::Future {
        std::future::ready(Err(TestError))
    }
}

#[tokio::test]
async fn reset_command_closes_open_breaker() {
    let registry = CircuitBreakerRegistry::default();

    let cfg =
        CircuitBreakerConfig::new(1, Duration::from_millis(1), 1).unwrap().with_id("cb_reset");
    let layer = CircuitBreakerLayer::new(cfg).unwrap().with_registry(registry.clone());
    let mut svc = layer.layer(FailingSvc);

    // Trigger an error to open the breaker.
    let _ = svc.ready().await.unwrap().call(()).await;
    let state = registry.get("cb_reset").unwrap().state();
    assert_eq!(state, CircuitState::Open);

    // Execute reset command and verify state closes.
    let handler = BuiltInHandler::default().with_circuit_breaker_registry(registry.clone());
    let mut auth = AuthRegistry::new(AuthMode::First);
    auth.register(std::sync::Arc::new(PassthroughAuth));
    let history = Arc::new(InMemoryHistory::default());
    let router = CommandRouter::new(auth, Arc::new(handler), history);

    let env = CommandEnvelope {
        cmd: BuiltInCommand::ResetCircuitBreaker { id: "cb_reset".into() },
        auth: Some(AuthPayload::Opaque(vec![])),
        meta: CommandMeta { id: "3".into(), correlation_id: None, timestamp_millis: None },
    };

    let res = router.execute(env).await.unwrap();
    assert_eq!(res, CommandResult::Ack);

    let state = registry.get("cb_reset").unwrap().state();
    assert_eq!(state, CircuitState::Closed);
}

#[derive(Debug, Clone, thiserror::Error)]
#[error("test")]
struct TestError;