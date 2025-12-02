#![allow(missing_docs)]

use ninelives::adaptive::Adaptive;
use ninelives::circuit_breaker_registry::{CircuitBreakerRegistry, DefaultCircuitBreakerRegistry};
use ninelives::control::{
    AuthContext, AuthMode, AuthPayload, AuthRegistry, CommandEnvelope, CommandError, CommandMeta,
    CommandResult, DefaultConfigRegistry, InMemoryHistory,
};
use ninelives::control::{
    BuiltInHandler, CommandHandler, CommandRouter, PassthroughAuth,
    WriteConfigCommand, ReadConfigCommand, ListConfigCommand, ResetCircuitBreakerCommand,
    GetStateCommand,
};
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

    let registry = DefaultConfigRegistry::new();
    registry.register_fromstr("max_attempts", adapt.clone());

    let handler = BuiltInHandler::default().with_config_registry(registry);
    let mut auth = AuthRegistry::new(AuthMode::First);
    auth.register(std::sync::Arc::new(PassthroughAuth));
    let history = std::sync::Arc::new(InMemoryHistory::default());
    let router = CommandRouter::new(auth, std::sync::Arc::new(handler), history);

    let env = CommandEnvelope {
        cmd: Box::new(WriteConfigCommand { path: "max_attempts".into(), value: "3".into() }),
        auth: Some(AuthPayload::Opaque(vec![])),
        meta: CommandMeta { id: "1".into(), correlation_id: None, timestamp_millis: None },
    };
    let res = router.execute(env).await.unwrap();
    assert_eq!(res, CommandResult::Ack);
    assert_eq!(*adapt.get(), 3);
}

#[tokio::test]
async fn read_config_without_registry_returns_error_variant() {
    let handler = BuiltInHandler::default();
    let env = CommandEnvelope {
        cmd: Box::new(ReadConfigCommand { path: "missing".into() }),
        auth: None,
        meta: CommandMeta::default(),
    };
    let err = handler
        .handle(
            env,
            AuthContext { principal: "p".into(), provider: "test", attributes: Default::default() },
        )
        .await;
    assert!(matches!(err, Err(CommandError::ConfigRegistryMissing { .. })));
}

#[tokio::test]
async fn list_config_returns_registered_keys() {
    let registry = DefaultConfigRegistry::new();
    registry.register_fromstr("max_attempts", Adaptive::new(1usize));
    registry.register_fromstr("timeout_ms", Adaptive::new(100usize));

    let handler = BuiltInHandler::default().with_config_registry(registry);
    let mut auth = AuthRegistry::new(AuthMode::First);
    auth.register(std::sync::Arc::new(PassthroughAuth));
    let history = std::sync::Arc::new(InMemoryHistory::default());
    let router = CommandRouter::new(auth, std::sync::Arc::new(handler), history);

    let env = CommandEnvelope {
        cmd: Box::new(ListConfigCommand),
        auth: Some(AuthPayload::Opaque(vec![])),
        meta: CommandMeta { id: "lc".into(), correlation_id: None, timestamp_millis: None },
    };
    let res = router.execute(env).await.unwrap();
    if let CommandResult::List(mut keys) = res {
        keys.sort();
        assert_eq!(keys, vec!["max_attempts", "timeout_ms"]);
    } else {
        panic!("expected List result");
    }
}

#[tokio::test]
async fn get_state_reports_open_breaker() {
    let registry = DefaultCircuitBreakerRegistry::default();

    // Create breaker and force it to open.
    let cfg = CircuitBreakerConfig::builder()
        .failure_threshold(1)
        .recovery_timeout(Duration::from_millis(1))
        .half_open_limit(1)
        .build()
        .unwrap()
        .with_id("cb_state");
    let layer = CircuitBreakerLayer::new(cfg).unwrap().with_registry(registry.clone());
    let mut svc = layer.layer(FailingSvc);
    let _ = svc.ready().await.unwrap().call(()).await;

    let handler = BuiltInHandler::default().with_circuit_breaker_registry(registry);
    let mut auth = AuthRegistry::new(AuthMode::First);
    auth.register(std::sync::Arc::new(PassthroughAuth));
    let history = Arc::new(InMemoryHistory::default());
    let router = CommandRouter::new(auth, Arc::new(handler), history);

    let env = CommandEnvelope {
        cmd: Box::new(GetStateCommand),
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
    let registry = DefaultCircuitBreakerRegistry::default();
    registry.register_new("cb1".into());

    let handler = BuiltInHandler::default().with_circuit_breaker_registry(registry);
    let mut auth = AuthRegistry::new(AuthMode::First);
    auth.register(std::sync::Arc::new(PassthroughAuth));
    let history = Arc::new(InMemoryHistory::default());
    let router = CommandRouter::new(auth, Arc::new(handler), history);

    let env = CommandEnvelope {
        cmd: Box::new(ResetCircuitBreakerCommand { id: "cb1".into() }),
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
    let registry = DefaultCircuitBreakerRegistry::default();

    let cfg = CircuitBreakerConfig::builder()
        .failure_threshold(1)
        .recovery_timeout(Duration::from_millis(1))
        .half_open_limit(1)
        .build()
        .unwrap()
        .with_id("cb_reset");
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
        cmd: Box::new(ResetCircuitBreakerCommand { id: "cb_reset".into() }),
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
