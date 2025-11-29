//! Control plane primitives: command envelope, auth, history, router.
//!
//! This is a lightweight, transport-agnostic control plane. Transports populate
//! `CommandEnvelope` with an `AuthPayload`; the router dispatches to handlers
//! after auth. History storage is pluggable.

/// Transport abstractions.
pub mod transport;
/// Channel-based transport implementation.
pub mod transport_channel;

use crate::circuit_breaker_registry::CircuitBreakerRegistry;
use std::collections::HashMap;
use std::fmt::Display;
use std::sync::Arc;

use async_trait::async_trait;
use futures::future::BoxFuture;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use tokio::sync::Mutex;
use tower::Service;
use tracing::info;

/// Opaque command identifier.
pub type CommandId = String;
/// Execution metadata attached to each command.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CommandMeta {
    /// Command identifier (unique per request).
    pub id: CommandId,
    /// Optional correlation ID for tracing.
    pub correlation_id: Option<String>,
    /// Timestamp in milliseconds (epoch).
    pub timestamp_millis: Option<u128>,
}

/// Auth payload sent alongside a command. Transports set this; providers verify it.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum AuthPayload {
    /// JSON Web Token authentication.
    Jwt {
        /// The JWT token string.
        token: String,
    },
    /// Cryptographic signatures payload.
    Signatures {
        /// SHA-256 hash of the payload.
        payload_hash: [u8; 32],
        /// List of detached signatures.
        signatures: Vec<DetachedSig>,
    },
    /// Mutual TLS authentication.
    Mtls {
        /// Peer Distinguished Name.
        peer_dn: String,
        /// Certificate chain (DER encoded).
        cert_chain: Vec<Vec<u8>>,
    },
    /// Opaque authentication payload (custom/fallback).
    Opaque(Vec<u8>),
}

/// Detached signature placeholder (payload-agnostic). Extend as needed.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DetachedSig {
    /// Algorithm used (e.g., "ed25519", "es256").
    pub algorithm: String,
    /// The signature bytes.
    pub signature: Vec<u8>,
    /// Optional key identifier (kid).
    pub key_id: Option<String>,
}

/// Command envelope carrying the command, auth payload, and metadata.
#[derive(Clone, Debug)]
pub struct CommandEnvelope<C: Clone> {
    /// The command payload.
    pub cmd: C,
    /// Authentication payload.
    pub auth: Option<AuthPayload>,
    /// Command metadata.
    pub meta: CommandMeta,
}

/// Command payload schema used by transports and handlers.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CommandContext {
    /// Command ID.
    pub id: String,
    /// Arguments for the command.
    #[serde(default)]
    pub args: JsonValue,
    /// Identity of the caller (if known/extracted).
    #[serde(default)]
    pub identity: Option<String>,
    /// Optional response channel ID.
    #[serde(default)]
    pub response_channel: Option<String>,
}

/// Result of authentication.
#[derive(Clone, Debug)]
pub struct AuthContext {
    /// Authenticated principal (user/service ID).
    pub principal: String,
    /// Name of the provider that authenticated the request.
    pub provider: &'static str,
    /// Additional attributes from the auth provider (claims, roles, etc.).
    pub attributes: HashMap<String, String>,
}

/// Errors produced during authentication or authorization.
#[derive(thiserror::Error, Debug)]
pub enum AuthError {
    /// Credentials missing or invalid.
    #[error("unauthenticated: {0}")]
    Unauthenticated(String),
    /// Authenticated but permission denied.
    #[error("unauthorized: {0}")]
    Unauthorized(String),
    /// Internal error in auth provider.
    #[error("internal auth error: {0}")]
    Internal(String),
}

/// Pluggable authentication/authorization provider.
pub trait AuthProvider: Send + Sync {
    /// Unique name of this provider.
    fn name(&self) -> &'static str;

    /// Verify credentials; returns context on success.
    fn authenticate(
        &self,
        meta: &CommandMeta,
        auth: Option<&AuthPayload>,
    ) -> Result<AuthContext, AuthError>;

    /// Optional authorization using the command label.
    fn authorize(
        &self,
        _ctx: &AuthContext,
        _label: &str,
        _meta: &CommandMeta,
    ) -> Result<(), AuthError> {
        Ok(())
    }
}

/// Registry that tries providers in order.
#[derive(Clone)]
pub struct AuthRegistry {
    providers: Vec<Arc<dyn AuthProvider>>,
    mode: AuthMode,
}

/// Strategy for combining multiple auth providers.
#[derive(Clone, Copy, Debug)]
pub enum AuthMode {
    /// First provider that authenticates wins.
    First,
    /// All providers must succeed.
    All,
}

impl AuthRegistry {
    /// Create a new registry with the given mode.
    pub fn new(mode: AuthMode) -> Self {
        Self { providers: Vec::new(), mode }
    }

    /// Register an auth provider.
    pub fn register(&mut self, provider: Arc<dyn AuthProvider>) {
        self.providers.push(provider);
    }

    /// Authenticate a command envelope using registered providers.
    pub fn authenticate<C>(&self, env: &CommandEnvelope<C>) -> Result<AuthContext, AuthError>
    where
        C: CommandLabel + Clone,
    {
        match self.mode {
            AuthMode::First => {
                let mut last_err = None;
                for p in &self.providers {
                    match p.authenticate(&env.meta, env.auth.as_ref()) {
                        Ok(ctx) => {
                            // If a provider authenticates but denies authorization, stop immediately
                            // to prevent later providers from overriding an explicit deny.
                            match p.authorize(&ctx, env.cmd.label(), &env.meta) {
                                Ok(()) => return Ok(ctx),
                                Err(e) => return Err(e),
                            }
                        }
                        Err(e) => last_err = Some(e),
                    }
                }
                Err(last_err.unwrap_or(AuthError::Unauthenticated("no providers".into())))
            }
            AuthMode::All => {
                let mut last_ctx = None;
                for p in &self.providers {
                    let ctx = p.authenticate(&env.meta, env.auth.as_ref())?;
                    p.authorize(&ctx, env.cmd.label(), &env.meta)?;
                    last_ctx = Some(ctx);
                }
                last_ctx.ok_or(AuthError::Unauthenticated("no providers".into()))
            }
        }
    }
}

/// Authorization layer wraps an inner service and preserves auth registry for later use.
#[derive(Clone)]
pub struct AuthorizationLayer {
    registry: Arc<AuthRegistry>,
}

impl AuthorizationLayer {
    /// Create a new authorization layer with the given registry.
    pub fn new(registry: AuthRegistry) -> Self {
        Self { registry: Arc::new(registry) }
    }
}

/// Service that applies authorization checks before forwarding commands.
#[derive(Clone)]
pub struct AuthorizationService<S> {
    inner: S,
    registry: Arc<AuthRegistry>,
}

impl<S> tower_layer::Layer<S> for AuthorizationLayer {
    type Service = AuthorizationService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        AuthorizationService { inner, registry: self.registry.clone() }
    }
}

impl<S, C> Service<CommandEnvelope<C>> for AuthorizationService<S>
where
    C: CommandLabel + Clone + Send + Sync + 'static,
    S: Service<CommandEnvelope<C>, Response = CommandResult, Error = CommandError>
        + Clone
        + Send
        + 'static,
    S::Future: Send + 'static,
{
    type Response = CommandResult;
    type Error = CommandError;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), CommandError>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: CommandEnvelope<C>) -> Self::Future {
        let registry = self.registry.clone();
        let mut inner = self.inner.clone();
        Box::pin(async move {
            registry.authenticate(&req).map_err(CommandError::Auth)?;
            inner.call(req).await
        })
    }
}

/// Passthrough provider (dev/testing).
pub struct PassthroughAuth;
impl AuthProvider for PassthroughAuth {
    fn name(&self) -> &'static str {
        "passthrough"
    }
    fn authenticate(
        &self,
        _meta: &CommandMeta,
        _auth: Option<&AuthPayload>,
    ) -> Result<AuthContext, AuthError> {
        Ok(AuthContext {
            principal: "anonymous".into(),
            provider: self.name(),
            attributes: HashMap::new(),
        })
    }
}

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

/// Errors returned by command handling.
#[derive(thiserror::Error, Debug)]
pub enum CommandError {
    /// Authentication or authorization failed.
    #[error("auth: {0}")]
    Auth(#[from] AuthError),
    /// Handler execution failed.
    #[error("handler: {0}")]
    Handler(String),
    /// Missing config registry when requested by a command.
    #[error("config registry missing: {hint}")]
    ConfigRegistryMissing {
        /// Guidance on how to wire the registry into the control builder.
        hint: &'static str,
    },
    /// Audit recording failed.
    #[error("audit: {0}")]
    Audit(String),
}

/// Structured command failure payload.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CommandFailure {
    /// Caller provided invalid arguments.
    InvalidArgs { msg: String },
    /// Requested resource was not found.
    NotFound { what: String },
    /// Required registry dependency missing.
    RegistryMissing { hint: String },
    /// Catch-all internal error.
    Internal { msg: String },
}

impl std::fmt::Display for CommandFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CommandFailure::InvalidArgs { msg } => write!(f, "{msg}"),
            CommandFailure::NotFound { what } => write!(f, "{what} not found"),
            CommandFailure::RegistryMissing { hint } => write!(f, "registry missing: {hint}"),
            CommandFailure::Internal { msg } => write!(f, "{msg}"),
        }
    }
}

/// Command result type.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum CommandResult {
    /// Command acknowledged (success).
    Ack,
    /// Command returned a value.
    Value(String),
    /// Command returned a list of values.
    List(Vec<String>),
    /// Reset complete.
    Reset,
    /// Error message.
    Error(CommandFailure),
}

/// Audit record emitted after command execution.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AuditRecord {
    /// Command ID.
    pub id: CommandId,
    /// Command label.
    pub label: String,
    /// Principal who executed the command.
    pub principal: String,
    /// Status/Result of execution.
    pub status: String,
}

/// Audit sink interface.
#[async_trait]
pub trait AuditSink: Send + Sync {
    /// Record an audit event.
    async fn record(&self, record: AuditRecord) -> Result<(), CommandError>;
}

/// Simple audit sink that logs via tracing.
pub struct TracingAuditSink;

#[async_trait]
impl AuditSink for TracingAuditSink {
    async fn record(&self, record: AuditRecord) -> Result<(), CommandError> {
        info!(target: "ninelives::audit", id=%record.id, label=%record.label, principal=%record.principal, status=%record.status, "audit");
        Ok(())
    }
}

/// Command history interface (pluggable storage).
#[async_trait]
pub trait CommandHistory: Send + Sync {
    /// Append a command execution record.
    async fn append(&self, meta: &CommandMeta, result: &CommandResult);
    /// List recent command history.
    async fn list(&self) -> Vec<CommandMeta>;
    /// Clear history.
    async fn clear(&self);
}

/// In-memory history (for tests / defaults).
#[derive(Default, Clone)]
pub struct InMemoryHistory {
    entries: Arc<Mutex<Vec<CommandMeta>>>,
}

#[async_trait]
impl CommandHistory for InMemoryHistory {
    async fn append(&self, meta: &CommandMeta, _result: &CommandResult) {
        self.entries.lock().await.push(meta.clone());
    }

    async fn list(&self) -> Vec<CommandMeta> {
        self.entries.lock().await.clone()
    }

    async fn clear(&self) {
        self.entries.lock().await.clear();
    }
}

/// In-memory audit sink (tests/diagnostics).
#[derive(Default)]
pub struct MemoryAuditSink {
    records: Arc<Mutex<Vec<AuditRecord>>>,
}

impl MemoryAuditSink {
    /// Create a new in-memory audit sink.
    pub fn new() -> Self {
        Self::default()
    }
    /// Retrieve recorded audit records.
    pub async fn records(&self) -> Vec<AuditRecord> {
        self.records.lock().await.clone()
    }
}

#[async_trait]
impl AuditSink for MemoryAuditSink {
    async fn record(&self, record: AuditRecord) -> Result<(), CommandError> {
        let mut guard = self.records.lock().await;
        guard.push(record);
        Ok(())
    }
}

/// Simple in-process router using an AuthRegistry and handler.
pub struct CommandRouter<C> {
    auth: AuthRegistry,
    handler: Arc<dyn CommandHandler<C>>,
    history: Arc<dyn CommandHistory>,
    audit: Option<Arc<dyn AuditSink>>,
}

impl<C> CommandRouter<C>
where
    C: Send + Sync + Clone + CommandLabel + 'static,
{
    /// Create a new command router.
    pub fn new(
        auth: AuthRegistry,
        handler: Arc<dyn CommandHandler<C>>,
        history: Arc<dyn CommandHistory>,
    ) -> Self {
        Self { auth, handler, history, audit: None }
    }

    /// Attach an audit sink to the router.
    pub fn with_audit(mut self, audit: Arc<dyn AuditSink>) -> Self {
        self.audit = Some(audit);
        self
    }

    /// Execute a command envelope through the router (auth -> handler -> history/audit).
    pub async fn execute(&self, env: CommandEnvelope<C>) -> Result<CommandResult, CommandError> {
        // Auth path: audit denials too.
        let auth_result = self.auth.authenticate(&env);
        let ctx = match auth_result {
            Ok(ctx) => ctx,
            Err(e) => {
                if let Some(sink) = &self.audit {
                    let record = AuditRecord {
                        id: env.meta.id.clone(),
                        label: env.cmd.label().into(),
                        principal: "unknown".into(),
                        status: format!("denied: {}", e),
                    };
                    sink.record(record).await?;
                }
                return Err(CommandError::Auth(e));
            }
        };

        let res = self.handler.handle(env.clone(), ctx.clone()).await?;
        self.history.append(&env.meta, &res).await;

        if let Some(sink) = &self.audit {
            let status = match &res {
                CommandResult::Error(e) => format!("error: {}", e),
                _ => "ok".into(),
            };
            let record = AuditRecord {
                id: env.meta.id.clone(),
                label: env.cmd.label().into(),
                principal: ctx.principal,
                status,
            };
            sink.record(record).await?;
        }
        Ok(res)
    }
}

// -----------------------------------------------------------------------------
// Example command + handler for in-process control plane
// -----------------------------------------------------------------------------

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

/// Trait for getting a string label for a command type.
pub trait CommandLabel {
    /// Returns the label for the command.
    fn label(&self) -> &str;
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

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use futures::future::ready;
    use tower::Service;

    fn env(cmd: BuiltInCommand) -> CommandEnvelope<BuiltInCommand> {
        CommandEnvelope {
            cmd,
            auth: Some(AuthPayload::Opaque(vec![])),
            meta: CommandMeta { id: "cmd-1".into(), correlation_id: None, timestamp_millis: None },
        }
    }

    #[tokio::test]
    async fn passthrough_auth_and_history() {
        let mut reg = AuthRegistry::new(AuthMode::First);
        reg.register(Arc::new(PassthroughAuth));

        let handler = Arc::new(BuiltInHandler::default());
        let history: Arc<dyn CommandHistory> = Arc::new(InMemoryHistory::default());
        let router = CommandRouter::new(reg, handler, history.clone());

        let res = router
            .execute(env(BuiltInCommand::Set { key: "k".into(), value: "v".into() }))
            .await
            .unwrap();
        assert_eq!(res, CommandResult::Ack);

        let res = router.execute(env(BuiltInCommand::Get { key: "k".into() })).await.unwrap();
        assert_eq!(res, CommandResult::Value("v".into()));

        let hist = history.list().await;
        assert_eq!(hist.len(), 2);
    }

    struct DenyAll;
    impl AuthProvider for DenyAll {
        fn name(&self) -> &'static str {
            "deny"
        }
        fn authenticate(
            &self,
            _meta: &CommandMeta,
            _auth: Option<&AuthPayload>,
        ) -> Result<AuthContext, AuthError> {
            Err(AuthError::Unauthenticated("nope".into()))
        }
    }

    #[tokio::test]
    async fn auth_mode_first_stops_on_success() {
        let mut reg = AuthRegistry::new(AuthMode::First);
        reg.register(Arc::new(DenyAll));
        reg.register(Arc::new(PassthroughAuth));

        let handler = Arc::new(BuiltInHandler::default());
        let history: Arc<dyn CommandHistory> = Arc::new(InMemoryHistory::default());
        let router = CommandRouter::new(reg, handler, history.clone());

        let res = router.execute(env(BuiltInCommand::List)).await.unwrap();
        assert_eq!(res, CommandResult::List(vec![]));
    }

    #[test]
    fn command_service_compiles() {
        struct EchoSvc;
        impl Service<CommandContext> for EchoSvc {
            type Response = CommandResult;
            type Error = CommandError;
            type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;
            fn poll_ready(
                &mut self,
                _cx: &mut std::task::Context<'_>,
            ) -> std::task::Poll<Result<(), Self::Error>> {
                std::task::Poll::Ready(Ok(()))
            }
            fn call(&mut self, _req: CommandContext) -> Self::Future {
                Box::pin(ready(Ok(CommandResult::Ack)))
            }
        }
        let _svc: Box<dyn CommandService> = Box::new(EchoSvc);
    }

    #[test]
    fn config_registry_keys_sorted() {
        let mut reg = DefaultConfigRegistry::new();
        reg.register_fromstr("b", crate::adaptive::Adaptive::new(1usize));
        reg.register_fromstr("a", crate::adaptive::Adaptive::new(2usize));
        assert_eq!(reg.keys(), vec!["a".to_string(), "b".to_string()]);
    }

    #[test]
    fn breaker_snapshot_sorted_and_states() {
        let reg = crate::circuit_breaker_registry::DefaultCircuitBreakerRegistry::default();
        reg.register_new("cb_a".into());
        reg.register_new("cb_b".into());
        let snap = reg.snapshot();
        assert_eq!(snap.len(), 2);
        assert_eq!(snap[0].0, "cb_a");
        assert_eq!(snap[0].1, crate::circuit_breaker::CircuitState::Closed);
        assert_eq!(snap[1].0, "cb_b");
    }
}
