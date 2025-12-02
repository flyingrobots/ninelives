//! Built-in command types for the control plane.
//!
//! These commands provide core administrative functionality like configuration
//! management, circuit breaker control, and system state inspection.

use super::command::Command;
use std::any::Any;

// =============================================================================
// Store Commands
// =============================================================================

/// Set a key-value pair in the store.
#[derive(Clone, Debug, PartialEq)]
pub struct SetCommand {
    /// Key to set.
    pub key: String,
    /// Value to set.
    pub value: String,
}

impl Command for SetCommand {
    fn label(&self) -> &str {
        "set"
    }

    fn clone_box(&self) -> Box<dyn Command> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Get a value from the store.
#[derive(Clone, Debug, PartialEq)]
pub struct GetCommand {
    /// Key to get.
    pub key: String,
}

impl Command for GetCommand {
    fn label(&self) -> &str {
        "get"
    }

    fn clone_box(&self) -> Box<dyn Command> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// List all keys in the store.
#[derive(Clone, Debug, PartialEq)]
pub struct ListCommand;

impl Command for ListCommand {
    fn label(&self) -> &str {
        "list"
    }

    fn clone_box(&self) -> Box<dyn Command> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Reset the store.
#[derive(Clone, Debug, PartialEq)]
pub struct ResetCommand;

impl Command for ResetCommand {
    fn label(&self) -> &str {
        "reset"
    }

    fn clone_box(&self) -> Box<dyn Command> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

// =============================================================================
// Config Commands
// =============================================================================

/// Read a configuration value.
#[derive(Clone, Debug, PartialEq)]
pub struct ReadConfigCommand {
    /// Config path.
    pub path: String,
}

impl Command for ReadConfigCommand {
    fn label(&self) -> &str {
        "read_config"
    }

    fn clone_box(&self) -> Box<dyn Command> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Write a configuration value.
#[derive(Clone, Debug, PartialEq)]
pub struct WriteConfigCommand {
    /// Config path.
    pub path: String,
    /// New value.
    pub value: String,
}

impl Command for WriteConfigCommand {
    fn label(&self) -> &str {
        "write_config"
    }

    fn clone_box(&self) -> Box<dyn Command> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// List all registered config keys.
#[derive(Clone, Debug, PartialEq)]
pub struct ListConfigCommand;

impl Command for ListConfigCommand {
    fn label(&self) -> &str {
        "list_config"
    }

    fn clone_box(&self) -> Box<dyn Command> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

// =============================================================================
// Circuit Breaker Commands
// =============================================================================

/// Reset a circuit breaker to closed state.
#[derive(Clone, Debug, PartialEq)]
pub struct ResetCircuitBreakerCommand {
    /// Breaker ID.
    pub id: String,
}

impl Command for ResetCircuitBreakerCommand {
    fn label(&self) -> &str {
        "reset_circuit_breaker"
    }

    fn clone_box(&self) -> Box<dyn Command> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

// =============================================================================
// System Commands
// =============================================================================

/// Get system state snapshot.
#[derive(Clone, Debug, PartialEq)]
pub struct GetStateCommand;

impl Command for GetStateCommand {
    fn label(&self) -> &str {
        "get_state"
    }

    fn clone_box(&self) -> Box<dyn Command> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Health check probe.
#[derive(Clone, Debug, PartialEq)]
pub struct HealthCommand;

impl Command for HealthCommand {
    fn label(&self) -> &str {
        "health"
    }

    fn clone_box(&self) -> Box<dyn Command> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
