use std::collections::HashMap;
use std::fs;
use std::path::Path;

use ninelives::adaptive::Adaptive;
use ninelives::control::{
    AuthPayload, BuiltInCommand, BuiltInHandler, CommandEnvelope, CommandMeta, CommandResult,
    DefaultConfigRegistry,
};

// Demonstrates snapshot/restore: load config from a JSON file, apply to the registry,
// then export it back out.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Expected JSON format, e.g.: {"retry.max_attempts":"3","timeout.ms":"1000"}
    let snapshot_path = Path::new("state.json");

    // 1) Build registry and register keys with parsing/formatting.
    let mut registry = DefaultConfigRegistry::new();
    registry.register_fromstr("retry.max_attempts", Adaptive::new(3usize));
    registry.register_fromstr("timeout.ms", Adaptive::new(1000usize));

    // 2) Load snapshot (if present) and apply.
    if snapshot_path.exists() {
        let data = fs::read_to_string(snapshot_path)?;
        let map: HashMap<String, String> = serde_json::from_str(&data)?;
        // In a real service you'd surface structured per-key errors; for brevity we collapse here.
        registry.apply_snapshot(map).map_err(|errs| errs.join(" | "))?;
        println!("Loaded snapshot from {}", snapshot_path.display());
    }

    // 3) Wire the handler with the hydrated registry.
    let handler = BuiltInHandler::default().with_config_registry(registry);

    // 4) Exercise a command to mutate state (write_config).
    // For demonstration purposes only. In production, configure a proper AuthProvider
    // (e.g., JWT/mTLS) as per SECURITY.md, and ensure AuthPayload is not Opaque.
    let env = CommandEnvelope {
        cmd: BuiltInCommand::WriteConfig { path: "retry.max_attempts".into(), value: "5".into() },
        auth: Some(AuthPayload::Opaque(vec![])),
        meta: CommandMeta { id: "1".into(), correlation_id: None, timestamp_millis: None },
    };
    handler.handle(env, Default::default()).await?;
    println!("Updated retry.max_attempts to 5");

    // 5) Export snapshot via GetState and persist.
    let state_env = CommandEnvelope {
        cmd: BuiltInCommand::GetState,
        auth: Some(AuthPayload::Opaque(vec![])),
        meta: CommandMeta { id: "2".into(), correlation_id: None, timestamp_millis: None },
    };
    let state = handler.handle(state_env, Default::default()).await?;
    match state {
        CommandResult::Value(s) => {
            fs::write(snapshot_path, s)?;
            println!("Persisted snapshot to {}", snapshot_path.display());
        }
        CommandResult::Ack => {
            return Err("GetState returned Ack without payload".into());
        }
        CommandResult::Error(fail) => {
            return Err(format!("GetState failed: {}", fail).into());
        }
        other => {
            return Err(format!("GetState returned unexpected result: {:?}", other).into());
        }
    }

    Ok(())
}
