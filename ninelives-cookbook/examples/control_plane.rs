use std::sync::Arc;
use std::time::Duration;

use ninelives::ChannelTransport;
use ninelives::control::{
    AuthMode, AuthPayload, AuthRegistry, BuiltInCommand, BuiltInHandler, CommandEnvelope,
    CommandHistory, CommandMeta, CommandResult, CommandRouter, DefaultConfigRegistry,
    InMemoryHistory, MemoryAuditSink, PassthroughAuth,
};
use ninelives::{Backoff, Jitter, RetryPolicy, TokioSleeper};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Build a retry policy and expose its adaptive knob
    let retry = RetryPolicy::<std::io::Error>::builder()
        .max_attempts(3)
        .backoff(Backoff::exponential(Duration::from_millis(50)))
        .with_jitter(Jitter::full())
        .with_sleeper(TokioSleeper::default())
        .build()?;
    let adaptive_attempts = retry.adaptive_max_attempts();
    println!("Initial retry.max_attempts = {}", adaptive_attempts.get());

    // Register adaptive knob with the config registry
    let mut cfg = DefaultConfigRegistry::new();
    cfg.register_fromstr("retry.max_attempts", adaptive_attempts);

    // Wire command router with passthrough auth, audit, and history
    let mut auth = AuthRegistry::new(AuthMode::First);
    auth.register(Arc::new(PassthroughAuth));

    let handler = Arc::new(BuiltInHandler::default().with_config_registry(cfg));
    let history: Arc<dyn CommandHistory> = Arc::new(InMemoryHistory::default());
    let audit = Arc::new(MemoryAuditSink::new());
    let router = Arc::new(CommandRouter::new(auth, handler, history).with_audit(audit));

    // In-process transport for demo purposes
    let transport = ChannelTransport::new(router.clone());

    // Write a new config value at runtime
    let write_cmd = CommandEnvelope {
        cmd: BuiltInCommand::WriteConfig { path: "retry.max_attempts".into(), value: "5".into() },
        auth: Some(AuthPayload::Opaque(vec![])),
        meta: CommandMeta { id: "cmd-write".into(), correlation_id: None, timestamp_millis: None },
    };
    let write_result = transport.send(write_cmd).await?;
    match write_result {
        CommandResult::Ack => println!("✓ Config write succeeded"),
        other => panic!("Expected Ack, got {:?}", other),
    }

    // Read it back to verify
    let read_cmd = CommandEnvelope {
        cmd: BuiltInCommand::ReadConfig { path: "retry.max_attempts".into() },
        auth: Some(AuthPayload::Opaque(vec![])),
        meta: CommandMeta { id: "cmd-read".into(), correlation_id: None, timestamp_millis: None },
    };
    match transport.send(read_cmd).await? {
        CommandResult::Value(val) => {
            assert_eq!(val, "5", "Expected value to be '5', got '{}'", val);
            println!("✓ retry.max_attempts is now {}", val);
            assert_eq!(adaptive_attempts.get(), 5, "Adaptive handle should reflect new value");
            println!("✓ Adaptive handle updated to {}", adaptive_attempts.get());
        }
        other => panic!("unexpected response: {:?}", other),
    }

    Ok(())
}
