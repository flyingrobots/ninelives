use super::types::*;
use crate::circuit_breaker_registry::CircuitBreakerRegistry;
use async_trait::async_trait;
use futures::future::BoxFuture;
use std::collections::HashMap;
use std::fmt::Display;
use std::sync::Arc;
use tower::Service;

/// Command handler trait.
#[async_trait]
pub trait CommandHandler<C: Clone>: Send + Sync {
    /// Handle an authenticated command.
    async fn handle(
        &self,
        cmd: CommandEnvelope<C>,
        ctx: AuthContext,
    ) -> Result<CommandResult, CommandError>;
}

/// Command service signature using tower::Service over CommandContext.
pub trait CommandService:
    Service<
        CommandContext,
        Response = CommandResult,
        Error = CommandError,
        Future = BoxFuture<'static, Result<CommandResult, CommandError>>,
    > + Send
    + Sync
{
}

impl<T> CommandService for T where
    T: Service<
            CommandContext,
            Response = CommandResult,
            Error = CommandError,
            Future = BoxFuture<'static, Result<CommandResult, CommandError>>,
        > + Send
        + Sync
{
}

/// Built-in control-plane command for testing/demo.
#[derive(Clone, Debug, PartialEq)]
pub enum BuiltInCommand {
    /// Set a value in the store.
    Set {
        /// Key to set.
        key: String,
        /// Value to set.
        value: String,
    },
    /// Get a value from the store.
    Get {
        /// Key to get.
        key: String,
    },
    /// List all keys in the store.
    List,
    /// Reset the store.
    Reset,
    /// Read a config value.
    ReadConfig {
        /// Config path.
        path: String,
    },
    /// Write a config value.
    WriteConfig {
        /// Config path.
        path: String,
        /// New value.
        value: String,
    },
    /// Reset a circuit breaker.
    ResetCircuitBreaker {
        /// Breaker ID.
        id: String,
    },
    /// List all registered config keys.
    ListConfig,
    /// Get system state snapshot.
    GetState,
    /// Health check probe.
    Health,
}

impl CommandLabel for BuiltInCommand {
    fn label(&self) -> &str {
        match self {
            BuiltInCommand::Set { .. } => "set",
            BuiltInCommand::Get { .. } => "get",
            BuiltInCommand::List => "list",
            BuiltInCommand::Reset => "reset",
            BuiltInCommand::ReadConfig { .. } => "read_config",
            BuiltInCommand::WriteConfig { .. } => "write_config",
            BuiltInCommand::ResetCircuitBreaker { .. } => "reset_circuit_breaker",
            BuiltInCommand::ListConfig => "list_config",
            BuiltInCommand::GetState => "get_state",
            BuiltInCommand::Health => "health",
        }
    }
}

/// Registry of live config bindings (Adaptive values).
pub trait ConfigRegistry: Send + Sync + std::fmt::Debug {
    /// Write a raw string into a registered config key.
    fn write(&self, path: &str, raw: &str) -> Result<(), String>;
    /// Read a rendered value for the given config key.
    fn read(&self, path: &str) -> Result<String, String>;
    /// List registered keys (sorted).
    fn keys(&self) -> Vec<String>;
    /// Check whether a key exists.
    fn contains(&self, path: &str) -> bool;
}

/// In-memory implementation of a config registry.
pub struct InMemoryConfigRegistry {
    entries: HashMap<String, Box<dyn ConfigEntry>>,
}

impl std::fmt::Debug for InMemoryConfigRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "InMemoryConfigRegistry{{entries:{}}}", self.entries.len())
    }
}

/// Default in-memory config registry implementation.
pub type DefaultConfigRegistry = InMemoryConfigRegistry;

impl Default for InMemoryConfigRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryConfigRegistry {
    /// Create a new config registry.
    pub fn new() -> Self {
        Self { entries: HashMap::new() }
    }

    /// Register a value using FromStr/Display for parsing/formatting.
    pub fn register_fromstr<T>(
        &mut self,
        path: impl Into<String>,
        handle: crate::adaptive::Adaptive<T>,
    ) where
        T: Clone + Send + Sync + 'static,
        T: std::str::FromStr,
        <T as std::str::FromStr>::Err: Display,
        T: Display,
    {
        self.register(
            path,
            handle,
            |raw| raw.parse::<T>().map_err(|e| format!("{}", e)),
            |v| format!("{}", v),
        );
    }

    /// Register with custom parse/render functions.
    pub fn register<T, P, R>(
        &mut self,
        path: impl Into<String>,
        handle: crate::adaptive::Adaptive<T>,
        parse: P,
        render: R,
    ) where
        T: Clone + Send + Sync + 'static,
        P: Fn(&str) -> Result<T, String> + Send + Sync + 'static,
        R: Fn(&T) -> String + Send + Sync + 'static,
    {
        self.entries.insert(
            path.into(),
            Box::new(GenericConfig { handle, parse: Arc::new(parse), render: Arc::new(render) }),
        );
    }

    /// Write a value to a registered config key.
    pub fn write(&self, path: &str, raw: &str) -> Result<(), String> {
        let entry = self.entries.get(path).ok_or_else(|| format!("unknown config path: {path}"))?;
        entry.write(raw)
    }

    /// Read a value from a registered config key.
    pub fn read(&self, path: &str) -> Result<String, String> {
        let entry = self.entries.get(path).ok_or_else(|| format!("unknown config path: {path}"))?;
        entry.read()
    }

    /// List registered config keys (sorted).
    pub fn keys(&self) -> Vec<String> {
        let mut keys: Vec<String> = self.entries.keys().cloned().collect();
        keys.sort();
        keys
    }

    /// Check whether a config key is registered.
    pub fn contains(&self, path: &str) -> bool {
        self.entries.contains_key(path)
    }
}

impl ConfigRegistry for InMemoryConfigRegistry {
    fn write(&self, path: &str, raw: &str) -> Result<(), String> {
        InMemoryConfigRegistry::write(self, path, raw)
    }
    fn read(&self, path: &str) -> Result<String, String> {
        InMemoryConfigRegistry::read(self, path)
    }
    fn keys(&self) -> Vec<String> {
        InMemoryConfigRegistry::keys(self)
    }
    fn contains(&self, path: &str) -> bool {
        InMemoryConfigRegistry::contains(self, path)
    }
}

trait ConfigEntry: Send + Sync {
    fn write(&self, raw: &str) -> Result<(), String>;
    fn read(&self) -> Result<String, String>;
}

type ParseFn<T> = Arc<dyn Fn(&str) -> Result<T, String> + Send + Sync>;
type RenderFn<T> = Arc<dyn Fn(&T) -> String + Send + Sync>;

struct GenericConfig<T> {
    handle: crate::adaptive::Adaptive<T>,
    parse: ParseFn<T>,
    render: RenderFn<T>,
}

impl<T> ConfigEntry for GenericConfig<T>
where
    T: Clone + Send + Sync + 'static,
{
    fn write(&self, raw: &str) -> Result<(), String> {
        let val = (self.parse)(raw)?;
        self.handle.set(val);
        Ok(())
    }

    fn read(&self) -> Result<String, String> {
        let val = self.handle.get();
        Ok((self.render)(&val))
    }
}

/// Async store service for built-in handler.
#[derive(Clone, Default)]
pub struct StoreService {
    inner: Arc<tokio::sync::RwLock<HashMap<String, String>>>,
}

impl StoreService {
    async fn insert(&self, key: String, value: String) {
        self.inner.write().await.insert(key, value);
    }

    async fn get(&self, key: &str) -> Option<String> {
        self.inner.read().await.get(key).cloned()
    }

    async fn keys(&self) -> Vec<String> {
        let mut keys: Vec<String> = self.inner.read().await.keys().cloned().collect();
        keys.sort();
        keys
    }

    async fn clear(&self) {
        self.inner.write().await.clear();
    }
}

/// Config service encapsulating registry access.
#[derive(Clone, Default)]
pub struct ConfigService {
    registry: Option<Arc<dyn ConfigRegistry>>,
}

impl ConfigService {
    /// Return a copy with a registry attached.
    pub fn with_registry<R: ConfigRegistry + 'static>(mut self, registry: R) -> Self {
        self.registry = Some(Arc::new(registry));
        self
    }

    /// Set the registry on an existing service.
    pub fn set_registry<R: ConfigRegistry + 'static>(&mut self, registry: R) {
        self.registry = Some(Arc::new(registry));
    }

    fn registry(&self) -> Option<&Arc<dyn ConfigRegistry>> {
        self.registry.as_ref()
    }

    fn registry_or_err(&self) -> Result<&Arc<dyn ConfigRegistry>, CommandError> {
        self.registry.as_ref().ok_or(CommandError::ConfigRegistryMissing {
            hint: "Inject via ControlBuilder::with_config_registry()",
        })
    }

    fn contains(&self, key: &str) -> bool {
        self.registry().map(|r| r.contains(key)).unwrap_or(false)
    }

    fn write(&self, path: &str, value: &str) -> Result<CommandResult, CommandError> {
        let reg = self.registry_or_err()?;
        match reg.write(path, value) {
            Ok(()) => Ok(CommandResult::Ack),
            Err(e) => Ok(CommandResult::Error(CommandFailure::InvalidArgs { msg: e })),
        }
    }

    fn read(&self, path: &str) -> Result<CommandResult, CommandError> {
        let reg = self.registry_or_err()?;
        Ok(match reg.read(path) {
            Ok(val) => CommandResult::Value(val),
            Err(e) => CommandResult::Error(CommandFailure::InvalidArgs { msg: e }),
        })
    }

    fn list(&self) -> Result<Vec<String>, CommandError> {
        let reg = self.registry_or_err()?;
        Ok(reg.keys())
    }
}

/// Circuit breaker service wrapper.
#[derive(Clone, Default)]
pub struct BreakerService {
    registry: Option<Arc<dyn CircuitBreakerRegistry>>,
}

impl BreakerService {
    /// Return a copy with a circuit breaker registry attached.
    pub fn with_registry<R: CircuitBreakerRegistry + 'static>(mut self, registry: R) -> Self {
        self.registry = Some(Arc::new(registry));
        self
    }

    fn registry(&self) -> Result<&Arc<dyn CircuitBreakerRegistry>, CommandError> {
        self.registry
            .as_ref()
            .ok_or(CommandError::Handler("circuit breaker registry not set".into()))
    }

    fn reset(&self, id: &str) -> Result<CommandResult, CommandError> {
        let reg = self.registry()?;
        match reg.reset(id) {
            Ok(()) => Ok(CommandResult::Ack),
            Err(e) => Ok(CommandResult::Error(CommandFailure::NotFound {
                what: format!("circuit_breaker:{id} ({e})"),
            })),
        }
    }

    fn snapshot(
        &self,
    ) -> Result<Vec<(String, crate::circuit_breaker::CircuitState)>, CommandError> {
        let reg = self.registry()?;
        Ok(reg.snapshot())
    }
}

/// Aggregated state/services for built-in commands.
#[derive(Clone, Default)]
pub struct ControlState {
    store: StoreService,
    config: ConfigService,
    breakers: BreakerService,
}

/// Built-in handler for basic commands.
#[derive(Clone, Default)]
pub struct BuiltInHandler {
    state: Arc<ControlState>,
}

impl BuiltInHandler {
    /// Attach a config registry to the handler.
    pub fn with_config_registry<R>(mut self, registry: R) -> Self
    where
        R: ConfigRegistry + 'static,
    {
        Arc::make_mut(&mut self.state).config.set_registry(registry);
        self
    }

    /// Attach a circuit breaker registry to the handler.
    pub fn with_circuit_breaker_registry<R>(mut self, registry: R) -> Self
    where
        R: CircuitBreakerRegistry + 'static,
    {
        Arc::make_mut(&mut self.state).breakers = BreakerService::default().with_registry(registry);
        self
    }

    /// Set the config registry.
    pub fn set_config_registry<R>(&mut self, registry: R)
    where
        R: ConfigRegistry + 'static,
    {
        Arc::make_mut(&mut self.state).config.set_registry(registry);
    }

    async fn handle_config(
        &self,
        cmd: &BuiltInCommand,
    ) -> Option<Result<CommandResult, CommandError>> {
        match cmd {
            BuiltInCommand::WriteConfig { path, value } => {
                Some(self.state.config.write(path, value))
            }
            BuiltInCommand::ListConfig => Some(self.state.config.list().map(CommandResult::List)),
            BuiltInCommand::ReadConfig { path } => Some(self.state.config.read(path)),
            _ => None,
        }
    }

    async fn handle_store(
        &self,
        cmd: &BuiltInCommand,
    ) -> Option<Result<CommandResult, CommandError>> {
        match cmd {
            BuiltInCommand::Set { key, value } => {
                Some(self.set_or_store(key.clone(), value.clone()).await)
            }
            BuiltInCommand::Get { key } => Some(Ok(self.get_from_store_or_config(key).await)),
            BuiltInCommand::List => {
                let store_keys: Vec<String> = self
                    .state
                    .store
                    .keys()
                    .await
                    .into_iter()
                    .map(|k| format!("store:{k}"))
                    .collect();
                let config_keys: Vec<String> = self
                    .state
                    .config
                    .registry()
                    .map(|reg| reg.keys().into_iter().map(|k| format!("config:{k}")))
                    .map(|iter| iter.collect())
                    .unwrap_or_default();
                let mut keys: Vec<String> = store_keys.into_iter().chain(config_keys).collect();
                keys.sort();
                Some(Ok(CommandResult::List(keys)))
            }
            BuiltInCommand::Reset => {
                self.state.store.clear().await;
                Some(Ok(CommandResult::Reset))
            }
            _ => None,
        }
    }

    async fn handle_breaker(
        &self,
        cmd: &BuiltInCommand,
    ) -> Option<Result<CommandResult, CommandError>> {
        match cmd {
            BuiltInCommand::ResetCircuitBreaker { id } => Some(self.state.breakers.reset(id)),
            BuiltInCommand::GetState => {
                let breakers = match self.state.breakers.snapshot() {
                    Ok(b) => b,
                    Err(e) => return Some(Err(e)),
                };
                let breaker_map: serde_json::Map<String, serde_json::Value> = breakers
                    .into_iter()
                    .map(|(id, state)| {
                        (
                            id,
                            serde_json::Value::String(
                                match state {
                                    crate::circuit_breaker::CircuitState::Closed => "Closed",
                                    crate::circuit_breaker::CircuitState::Open => "Open",
                                    crate::circuit_breaker::CircuitState::HalfOpen => "HalfOpen",
                                }
                                .into(),
                            ),
                        )
                    })
                    .collect();

                let mut config_map = serde_json::Map::new();
                if let Some(reg) = self.state.config.registry() {
                    for key in reg.keys() {
                        if let Ok(val) = reg.read(&key) {
                            config_map.insert(key, serde_json::Value::String(val));
                        }
                    }
                }

                let mut root = serde_json::Map::new();
                root.insert("breakers".into(), serde_json::Value::Object(breaker_map));
                root.insert("config".into(), serde_json::Value::Object(config_map));

                let res = serde_json::to_string(&root)
                    .map(CommandResult::Value)
                    .map_err(|e| CommandError::Handler(format!("failed to serialize state: {e}")));
                Some(res)
            }
            BuiltInCommand::Health => Some(Ok(CommandResult::Value(
                serde_json::json!({
                    "status": "ok",
                    "version": env!("CARGO_PKG_VERSION")
                })
                .to_string(),
            ))),
            _ => None,
        }
    }

    async fn set_or_store(
        &self,
        key: String,
        value: String,
    ) -> Result<CommandResult, CommandError> {
        if self.state.config.contains(&key) {
            return self.state.config.write(&key, &value);
        }
        self.state.store.insert(key, value).await;
        Ok(CommandResult::Ack)
    }

    /// Retrieves a value by checking the config registry first, then falling back to the
    /// async store. If neither contains the key, a default value (empty string for store,
    /// or error for config) is returned. This mirrors the precedence used by [`set_or_store`](Self::set_or_store)
    /// for consistency and maintainability.
    async fn get_from_store_or_config(&self, key: &str) -> CommandResult {
        if self.state.config.contains(key) {
            return self.state.config.read(key).unwrap_or(CommandResult::Error(
                CommandFailure::Internal { msg: "read failed".into() },
            ));
        }
        let val = self.state.store.get(key).await.unwrap_or_default();
        CommandResult::Value(val)
    }
}

#[async_trait]
impl CommandHandler<BuiltInCommand> for BuiltInHandler {
    async fn handle(
        &self,
        cmd: CommandEnvelope<BuiltInCommand>,
        _ctx: AuthContext,
    ) -> Result<CommandResult, CommandError> {
        if let Some(res) = self.handle_config(&cmd.cmd).await {
            return res;
        }
        if let Some(res) = self.handle_store(&cmd.cmd).await {
            return res;
        }
        if let Some(res) = self.handle_breaker(&cmd.cmd).await {
            return res;
        }
        Err(CommandError::Handler("unknown command".into()))
    }
}
