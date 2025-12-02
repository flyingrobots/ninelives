//! Command abstraction for the control plane.
//!
//! The `Command` trait is the primary extension point for adding custom commands
//! to the control plane. Commands are registered by label and dispatched dynamically
//! at runtime.

use std::any::Any;
use std::fmt::Debug;

/// Trait for commands that can be executed by the control plane.
///
/// This allows users to define custom command types beyond the built-in ones.
/// Each command must provide a unique label for routing and be fully self-contained.
///
/// # Implementation Requirements
///
/// - Must be `Clone + Send + Sync + Debug`
/// - Must provide a stable, unique label
/// - Must support downcasting via `as_any`
///
/// # Example
///
/// ```rust
/// use ninelives::control::command::Command;
/// use std::any::Any;
///
/// #[derive(Clone, Debug)]
/// struct ScaleService {
///     replicas: u32,
/// }
///
/// impl Command for ScaleService {
///     fn label(&self) -> &str {
///         "scale_service"
///     }
///
///     fn clone_box(&self) -> Box<dyn Command> {
///         Box::new(self.clone())
///     }
///
///     fn as_any(&self) -> &dyn Any {
///         self
///     }
/// }
/// ```
pub trait Command: Send + Sync + Debug {
    /// Returns a unique, human-readable label for the command.
    ///
    /// This label is used for:
    /// - Command routing and dispatch
    /// - Audit logging
    /// - Authorization checks
    ///
    /// Labels should be stable across versions and unique within a deployment.
    fn label(&self) -> &str;

    /// Clone this command into a Box<dyn Command>.
    ///
    /// This is required for the registry to work with trait objects.
    ///
    /// # Implementation
    ///
    /// ```ignore
    /// fn clone_box(&self) -> Box<dyn Command> {
    ///     Box::new(self.clone())
    /// }
    /// ```
    fn clone_box(&self) -> Box<dyn Command>;

    /// Provide access to the underlying type for downcasting.
    ///
    /// This enables type-safe extraction in handlers.
    ///
    /// # Implementation
    ///
    /// ```ignore
    /// fn as_any(&self) -> &dyn Any {
    ///     self
    /// }
    /// ```
    fn as_any(&self) -> &dyn Any;
}

// Enable cloning of Box<dyn Command>
impl Clone for Box<dyn Command> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

/// Factory for parsing wire-format commands into typed Command instances.
///
/// This trait enables the transport layer to convert from generic JSON/wire
/// representations into concrete command types.
pub trait CommandFactory: Send + Sync {
    /// Parse command arguments into a concrete Command instance.
    ///
    /// # Arguments
    ///
    /// * `label` - The command label from the wire format
    /// * `args` - JSON arguments for the command
    ///
    /// # Returns
    ///
    /// A boxed Command instance, or an error if parsing fails.
    fn create(&self, label: &str, args: &serde_json::Value) -> Result<Box<dyn Command>, String>;
}

/// Registry for mapping command labels to their factories.
///
/// This enables runtime extensibility - users can register custom command types
/// and their corresponding factories.
pub struct CommandRegistry {
    factories: std::sync::RwLock<std::collections::HashMap<String, Box<dyn CommandFactory>>>,
}

impl CommandRegistry {
    /// Create a new empty command registry.
    pub fn new() -> Self {
        Self {
            factories: std::sync::RwLock::new(std::collections::HashMap::new()),
        }
    }

    /// Register a factory for a given command label.
    ///
    /// If a factory for this label already exists, it will be replaced.
    pub fn register(&self, label: impl Into<String>, factory: Box<dyn CommandFactory>) {
        let mut factories = self.factories.write().expect("command registry lock poisoned");
        factories.insert(label.into(), factory);
    }

    /// Parse a command from wire format using registered factories.
    ///
    /// # Arguments
    ///
    /// * `label` - The command label
    /// * `args` - JSON arguments
    ///
    /// # Returns
    ///
    /// The parsed command, or an error if no factory is registered or parsing fails.
    pub fn parse(&self, label: &str, args: &serde_json::Value) -> Result<Box<dyn Command>, String> {
        let factories = self.factories.read().expect("command registry lock poisoned");
        let factory = factories
            .get(label)
            .ok_or_else(|| format!("unknown command: {}", label))?;
        factory.create(label, args)
    }

    /// List all registered command labels (sorted).
    pub fn labels(&self) -> Vec<String> {
        let factories = self.factories.read().expect("command registry lock poisoned");
        let mut labels: Vec<String> = factories.keys().cloned().collect();
        labels.sort();
        labels
    }

    /// Check if a command label is registered.
    pub fn contains(&self, label: &str) -> bool {
        let factories = self.factories.read().expect("command registry lock poisoned");
        factories.contains_key(label)
    }
}

impl Default for CommandRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for CommandRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let labels = self.labels();
        f.debug_struct("CommandRegistry")
            .field("registered_commands", &labels)
            .finish()
    }
}
