# Extensible Control Plane - Custom Commands Guide

The ninelives v0.3.0 control plane now supports custom commands via the `Command` trait.

## Quick Start: Defining a Custom Command

```rust
use ninelives::control::command::Command;
use std::any::Any;

#[derive(Clone, Debug)]
struct ScaleService {
    service_name: String,
    replicas: u32,
}

impl Command for ScaleService {
    fn label(&self) -> &str {
        "scale_service"
    }

    fn clone_box(&self) -> Box<dyn Command> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
```

## Creating a Factory

```rust
use ninelives::control::command::CommandFactory;
use serde_json::Value as JsonValue;

struct ScaleServiceFactory;

impl CommandFactory for ScaleServiceFactory {
    fn create(&self, label: &str, args: &JsonValue) -> Result<Box<dyn Command>, String> {
        let service_name = args["service_name"]
            .as_str()
            .ok_or("missing service_name")?
            .to_string();

        let replicas = args["replicas"]
            .as_u64()
            .ok_or("missing replicas")? as u32;

        Ok(Box::new(ScaleService { service_name, replicas }))
    }
}
```

## Implementing a Handler

```rust
use ninelives::control::{
    CommandHandler, CommandEnvelope, CommandResult, CommandError, AuthContext
};
use async_trait::async_trait;

struct MyCustomHandler;

#[async_trait]
impl CommandHandler for MyCustomHandler {
    async fn handle(
        &self,
        cmd: CommandEnvelope,
        _ctx: AuthContext,
    ) -> Result<CommandResult, CommandError> {
        match cmd.cmd.label() {
            "scale_service" => {
                let scale_cmd = cmd.cmd.as_any()
                    .downcast_ref::<ScaleService>()
                    .ok_or_else(|| CommandError::Handler("invalid command".into()))?;

                // Perform scaling operation
                println!("Scaling {} to {} replicas",
                         scale_cmd.service_name, scale_cmd.replicas);

                Ok(CommandResult::Ack)
            }
            _ => Err(CommandError::Handler(format!("unknown command: {}", cmd.cmd.label()))),
        }
    }
}
```

## Registering Commands

```rust
use ninelives::control::command::CommandRegistry as CommandTypeRegistry;

// Create registry
let registry = CommandTypeRegistry::new();

// Register your custom factory
registry.register("scale_service", Box::new(ScaleServiceFactory));

// Check what's registered
assert!(registry.contains("scale_service"));
let labels = registry.labels();  // ["scale_service"]
```

## Built-in Commands

The following built-in commands are available out of the box:

- **Store Operations**: `SetCommand`, `GetCommand`, `ListCommand`, `ResetCommand`
- **Config Management**: `ReadConfigCommand`, `WriteConfigCommand`, `ListConfigCommand`
- **Circuit Breakers**: `ResetCircuitBreakerCommand`
- **System**: `GetStateCommand`, `HealthCommand`

## Using Built-in Commands

```rust
use ninelives::control::{WriteConfigCommand, BuiltInCommandFactory};
use ninelives::control::command::CommandFactory;
use serde_json::json;

let factory = BuiltInCommandFactory;
let args = json!({"path": "max_retries", "value": "5"});
let cmd = factory.create("write_config", &args).unwrap();

assert_eq!(cmd.label(), "write_config");
```

## Architecture Benefits

### Before (v0.2.x)
- Commands were a hardcoded `enum`
- Users couldn't add custom commands
- Had to fork the library to extend

### After (v0.3.0)
- Commands are trait objects: `Box<dyn Command>`
- Runtime command registration via `CommandFactory`
- Users can add custom commands without modifying ninelives
- Built-in commands remain fully functional

## Key Types

```rust
// Main extension point
pub trait Command: Send + Sync + Debug {
    fn label(&self) -> &str;
    fn clone_box(&self) -> Box<dyn Command>;
    fn as_any(&self) -> &dyn Any;
}

// Factory for parsing commands from JSON
pub trait CommandFactory: Send + Sync {
    fn create(&self, label: &str, args: &JsonValue)
        -> Result<Box<dyn Command>, String>;
}

// Registry for runtime registration
pub struct CommandRegistry {
    // Maps label -> factory
}

// Handler processes commands
#[async_trait]
pub trait CommandHandler: Send + Sync {
    async fn handle(&self, cmd: CommandEnvelope, ctx: AuthContext)
        -> Result<CommandResult, CommandError>;
}
```

## Complete Example

See `ninelives-cookbook/examples/control_plane.rs` for a full working example.
