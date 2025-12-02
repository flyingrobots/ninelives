//! Factory for parsing built-in commands from JSON.

use super::builtin_commands::*;
use super::command::{Command, CommandFactory};
use serde_json::Value as JsonValue;

/// Factory for all built-in commands.
pub struct BuiltInCommandFactory;

impl CommandFactory for BuiltInCommandFactory {
    fn create(&self, label: &str, args: &JsonValue) -> Result<Box<dyn Command>, String> {
        match label {
            "set" => parse_set(args),
            "get" => parse_get(args),
            "list" => Ok(Box::new(ListCommand)),
            "reset" => Ok(Box::new(ResetCommand)),
            "read_config" => parse_read_config(args),
            "write_config" => parse_write_config(args),
            "list_config" => Ok(Box::new(ListConfigCommand)),
            "reset_circuit_breaker" => parse_reset_circuit_breaker(args),
            "get_state" => Ok(Box::new(GetStateCommand)),
            "health" => Ok(Box::new(HealthCommand)),
            _ => Err(format!("unknown built-in command: {}", label)),
        }
    }
}

// =============================================================================
// Parsing helpers
// =============================================================================

fn parse_set(args: &JsonValue) -> Result<Box<dyn Command>, String> {
    let key = args
        .get("key")
        .and_then(|v| v.as_str())
        .ok_or("missing 'key' field")?
        .to_string();
    let value = args
        .get("value")
        .and_then(|v| v.as_str())
        .ok_or("missing 'value' field")?
        .to_string();
    Ok(Box::new(SetCommand { key, value }))
}

fn parse_get(args: &JsonValue) -> Result<Box<dyn Command>, String> {
    let key = args
        .get("key")
        .and_then(|v| v.as_str())
        .ok_or("missing 'key' field")?
        .to_string();
    Ok(Box::new(GetCommand { key }))
}

fn parse_read_config(args: &JsonValue) -> Result<Box<dyn Command>, String> {
    let path = args
        .get("path")
        .and_then(|v| v.as_str())
        .ok_or("missing 'path' field")?
        .to_string();
    Ok(Box::new(ReadConfigCommand { path }))
}

fn parse_write_config(args: &JsonValue) -> Result<Box<dyn Command>, String> {
    let path = args
        .get("path")
        .and_then(|v| v.as_str())
        .ok_or("missing 'path' field")?
        .to_string();
    let value = args
        .get("value")
        .and_then(|v| v.as_str())
        .ok_or("missing 'value' field")?
        .to_string();
    Ok(Box::new(WriteConfigCommand { path, value }))
}

fn parse_reset_circuit_breaker(args: &JsonValue) -> Result<Box<dyn Command>, String> {
    let id = args
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or("missing 'id' field")?
        .to_string();
    Ok(Box::new(ResetCircuitBreakerCommand { id }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parse_set_command() {
        let factory = BuiltInCommandFactory;
        let args = json!({"key": "foo", "value": "bar"});
        let cmd = factory.create("set", &args).unwrap();
        assert_eq!(cmd.label(), "set");

        let set_cmd = cmd.as_any().downcast_ref::<SetCommand>().unwrap();
        assert_eq!(set_cmd.key, "foo");
        assert_eq!(set_cmd.value, "bar");
    }

    #[test]
    fn parse_get_command() {
        let factory = BuiltInCommandFactory;
        let args = json!({"key": "foo"});
        let cmd = factory.create("get", &args).unwrap();
        assert_eq!(cmd.label(), "get");

        let get_cmd = cmd.as_any().downcast_ref::<GetCommand>().unwrap();
        assert_eq!(get_cmd.key, "foo");
    }

    #[test]
    fn parse_list_command() {
        let factory = BuiltInCommandFactory;
        let cmd = factory.create("list", &json!({})).unwrap();
        assert_eq!(cmd.label(), "list");
    }

    #[test]
    fn parse_write_config_command() {
        let factory = BuiltInCommandFactory;
        let args = json!({"path": "retry.max_attempts", "value": "5"});
        let cmd = factory.create("write_config", &args).unwrap();
        assert_eq!(cmd.label(), "write_config");

        let write_cmd = cmd.as_any().downcast_ref::<WriteConfigCommand>().unwrap();
        assert_eq!(write_cmd.path, "retry.max_attempts");
        assert_eq!(write_cmd.value, "5");
    }

    #[test]
    fn parse_unknown_command_fails() {
        let factory = BuiltInCommandFactory;
        let err = factory.create("unknown", &json!({})).unwrap_err();
        assert!(err.contains("unknown"));
    }

    #[test]
    fn parse_set_missing_key_fails() {
        let factory = BuiltInCommandFactory;
        let args = json!({"value": "bar"});
        let err = factory.create("set", &args).unwrap_err();
        assert!(err.contains("key"));
    }
}
