//! Control plane primitives: command envelope, auth, history, router.
//!
//! This is a lightweight, transport-agnostic. Transports populate
//! `CommandEnvelope` with an `AuthPayload`; the router dispatches to handlers
//! after auth. History storage is pluggable.

/// Transport abstractions.
pub mod transport;
/// Channel-based transport implementation.
pub mod transport_channel;

use crate::circuit_breaker_registry::CircuitBreakerRegistry;
use std::collections::HashMap;
use std::fmt::Display;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use futures::future::BoxFuture;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use tower::Service;
use tracing::info;

/// Opaque command identifier.
pub type CommandId = String;
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CommandMeta {
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
}

impl<S> tower_layer::Layer<S> for AuthorizationLayer {
    type Service = AuthorizationService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        AuthorizationService { inner }
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
        let registry = self._registry.clone();
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
    /// Audit recording failed.
    #[error("audit: {0}")]
    Audit(String),
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
    Error(String),
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
        self.entries.lock().expect("history lock poisoned").push(meta.clone());
    }

    async fn list(&self) -> Vec<CommandMeta> {
        self.entries.lock().expect("history lock poisoned").clone()
    }

    async fn clear(&self) {
        self.entries.lock().expect("history lock poisoned").clear();
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
    pub fn records(&self) -> Vec<AuditRecord> {
        self.records.lock().expect("audit lock poisoned").clone()
    }
}

#[async_trait]
impl AuditSink for MemoryAuditSink {
    async fn record(&self, record: AuditRecord) -> Result<(), CommandError> {
        self.records.lock().expect("audit lock poisoned").push(record);
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
        }
    }
}

/// Registry of live config bindings (Adaptive values).
pub struct ConfigRegistry {
    entries: HashMap<String, Box<dyn ConfigEntry>>,
}

impl Default for ConfigRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfigRegistry {
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

/// Built-in handler for basic commands.
#[derive(Default)]
pub struct BuiltInHandler {
    store: Arc<Mutex<HashMap<String, String>>>,
    config_registry: Option<ConfigRegistry>,
    circuit_breaker_registry: Option<CircuitBreakerRegistry>,
}

impl BuiltInHandler {
    /// Attach a config registry to the handler.
    pub fn with_config_registry(mut self, registry: ConfigRegistry) -> Self {
        self.config_registry = Some(registry);
        self
    }

    /// Attach a circuit breaker registry to the handler.
    pub fn with_circuit_breaker_registry(mut self, registry: CircuitBreakerRegistry) -> Self {
        self.circuit_breaker_registry = Some(registry);
        self
    }

    /// Set the config registry.
    pub fn set_config_registry(&mut self, registry: ConfigRegistry) {
        self.config_registry = Some(registry);
    }

    fn config_registry(&self) -> Option<&ConfigRegistry> {
        self.config_registry.as_ref()
    }

    fn set_or_store(&self, key: String, value: String) -> Result<CommandResult, CommandError> {
        if let Some(reg) = self.config_registry() {
            if reg.contains(&key) {
                return match reg.write(&key, &value) {
                    Ok(()) => Ok(CommandResult::Ack),
                    Err(e) => Ok(CommandResult::Error(e)),
                };
            }
        }
        self.store.lock().unwrap().insert(key, value);
        Ok(CommandResult::Ack)
    }

    fn get_from_store_or_config(&self, key: &str) -> CommandResult {
        if let Some(reg) = self.config_registry() {
            if reg.contains(key) {
                return match reg.read(key) {
                    Ok(v) => CommandResult::Value(v),
                    Err(e) => CommandResult::Error(e),
                };
            }
        }
        let val = self.store.lock().unwrap().get(key).cloned().unwrap_or_default();
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
        match cmd.cmd {
            BuiltInCommand::Set { key, value } => self.set_or_store(key, value),
            BuiltInCommand::Get { key } => Ok(self.get_from_store_or_config(&key)),
            BuiltInCommand::List => {
                let store_keys =
                    self.store.lock().unwrap().keys().cloned().map(|k| format!("store:{k}"));
                let config_keys = self
                    .config_registry()
                    .map(|reg| reg.keys().into_iter().map(|k| format!("config:{k}")))
                    .into_iter()
                    .flatten();
                let mut keys: Vec<String> = store_keys.chain(config_keys).collect();
                keys.sort();
                keys.dedup();
                Ok(CommandResult::List(keys))
            }
            BuiltInCommand::Reset => {
                self.store.lock().unwrap().clear();
                Ok(CommandResult::Reset)
            }
            BuiltInCommand::WriteConfig { path, value } => {
                if let Some(reg) = self.config_registry() {
                    match reg.write(&path, &value) {
                        Ok(()) => Ok(CommandResult::Ack),
                        Err(e) => Ok(CommandResult::Error(e)),
                    }
                } else {
                    Ok(CommandResult::Error("config registry not set".into()))
                }
            }
            BuiltInCommand::ListConfig => {
                if let Some(reg) = self.config_registry() {
                    Ok(CommandResult::List(reg.keys()))
                } else {
                    Ok(CommandResult::Error("config registry not set".into()))
                }
            }
            BuiltInCommand::ReadConfig { path } => {
                if let Some(reg) = self.config_registry() {
                    Ok(match reg.read(&path) {
                        Ok(val) => CommandResult::Value(val),
                        Err(e) => CommandResult::Error(e),
                    })
                } else {
                    Ok(CommandResult::Error("config registry not set".into()))
                }
            }
            BuiltInCommand::ResetCircuitBreaker { id } => {
                if let Some(reg) = &self.circuit_breaker_registry {
                    match reg.reset(&id) {
                        Ok(()) => Ok(CommandResult::Ack),
                        Err(e) => Ok(CommandResult::Error(e)),
                    }
                } else {
                    Ok(CommandResult::Error("circuit breaker registry not set".into()))
                }
            }
            BuiltInCommand::GetState => {
                if let Some(reg) = &self.circuit_breaker_registry {
                    let breakers = reg.snapshot();
                    let map: serde_json::Map<String, serde_json::Value> = breakers
                        .into_iter()
                        .map(|(id, state)| {
                            (
                                id,
                                serde_json::Value::String(
                                    match state {
                                        crate::circuit_breaker::CircuitState::Closed => "Closed",
                                        crate::circuit_breaker::CircuitState::Open => "Open",
                                        crate::circuit_breaker::CircuitState::HalfOpen => {
                                            "HalfOpen"
                                        }
                                    }
                                    .into(),
                                ),
                            )
                        })
                        .collect();
                    let mut root = serde_json::Map::new();
                    root.insert("breakers".into(), serde_json::Value::Object(map));
                    match serde_json::to_string(&root) {
                        Ok(s) => Ok(CommandResult::Value(s)),
                        Err(e) => Ok(CommandResult::Error(format!(
                            "failed to serialize breaker state: {e}"
                        ))),
                    }
                } else {
                    Ok(CommandResult::Error(
                        "circuit breaker registry not set; cannot get state".into(),
                    ))
                }
            }
        }
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
        let mut reg = ConfigRegistry::new();
        reg.register_fromstr("b", crate::adaptive::Adaptive::new(1usize));
        reg.register_fromstr("a", crate::adaptive::Adaptive::new(2usize));
        assert_eq!(reg.keys(), vec!["a".to_string(), "b".to_string()]);
    }

    #[test]
    fn breaker_snapshot_sorted_and_states() {
        let reg = crate::circuit_breaker_registry::CircuitBreakerRegistry::default();
        reg.register_new("cb_a".into());
        reg.register_new("cb_b".into());
        let snap = reg.snapshot();
        assert_eq!(snap.len(), 2);
        assert_eq!(snap[0].0, "cb_a");
        assert_eq!(snap[0].1, crate::circuit_breaker::CircuitState::Closed);
        assert_eq!(snap[1].0, "cb_b");
    }
}
