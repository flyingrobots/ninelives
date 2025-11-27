//! Control plane primitives: command envelope, auth, history, router.
//!
//! This is a lightweight, transport-agnostic nucleus. Transports populate
//! `CommandEnvelope` with an `AuthPayload`; the router dispatches to handlers
//! after auth. History storage is pluggable.

use std::collections::HashMap;
use std::fmt::Display;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use futures::future::{self, BoxFuture};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use tower::Service;
use tracing::info;

/// Opaque command identifier.
pub type CommandId = String;

/// Metadata for commands (extensible).
#[derive(Clone, Debug, Default)]
pub struct CommandMeta {
    pub id: CommandId,
    pub correlation_id: Option<String>,
    pub timestamp_millis: Option<u128>,
}

/// Auth payload sent alongside a command. Transports set this; providers verify it.
#[derive(Clone, Debug)]
pub enum AuthPayload {
    Jwt { token: String },
    Signatures { payload_hash: [u8; 32], signatures: Vec<DetachedSig> },
    Mtls { peer_dn: String, cert_chain: Vec<Vec<u8>> },
    Opaque(Vec<u8>),
}

/// Detached signature placeholder (payload-agnostic). Extend as needed.
#[derive(Clone, Debug)]
pub struct DetachedSig {
    pub algorithm: String,
    pub signature: Vec<u8>,
    pub key_id: Option<String>,
}

/// Command envelope carrying the command, auth payload, and metadata.
#[derive(Clone, Debug)]
pub struct CommandEnvelope<C: Clone> {
    pub cmd: C,
    pub auth: Option<AuthPayload>,
    pub meta: CommandMeta,
}

/// Command payload schema used by transports and handlers.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CommandContext {
    pub id: String,
    #[serde(default)]
    pub args: JsonValue,
    #[serde(default)]
    pub identity: Option<String>,
    #[serde(default)]
    pub response_channel: Option<String>,
}

/// Result of authentication.
#[derive(Clone, Debug)]
pub struct AuthContext {
    pub principal: String,
    pub provider: &'static str,
    pub attributes: HashMap<String, String>,
}

#[derive(thiserror::Error, Debug)]
pub enum AuthError {
    #[error("unauthenticated: {0}")]
    Unauthenticated(String),
    #[error("unauthorized: {0}")]
    Unauthorized(String),
    #[error("internal auth error: {0}")]
    Internal(String),
}

/// Pluggable authentication/authorization provider.
pub trait AuthProvider: Send + Sync {
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
pub struct AuthRegistry {
    providers: Vec<Arc<dyn AuthProvider>>,
    mode: AuthMode,
}

#[derive(Clone, Copy, Debug)]
pub enum AuthMode {
    First, // first provider that authenticates wins
    All,   // all must succeed
}

impl AuthRegistry {
    pub fn new(mode: AuthMode) -> Self {
        Self { providers: Vec::new(), mode }
    }

    pub fn register(&mut self, provider: Arc<dyn AuthProvider>) {
        self.providers.push(provider);
    }

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
                            p.authorize(&ctx, env.cmd.label(), &env.meta)?;
                            return Ok(ctx);
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

#[derive(thiserror::Error, Debug)]
pub enum CommandError {
    #[error("auth: {0}")]
    Auth(#[from] AuthError),
    #[error("handler: {0}")]
    Handler(String),
    #[error("audit: {0}")]
    Audit(String),
}

#[derive(Clone, Debug, PartialEq)]
pub enum CommandResult {
    Ack,
    Value(String),
    List(Vec<String>),
    Reset,
    Error(String),
}

/// Audit record emitted after command execution.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AuditRecord {
    pub id: CommandId,
    pub label: String,
    pub principal: String,
    pub status: String,
}

/// Audit sink interface.
#[async_trait]
pub trait AuditSink: Send + Sync {
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
    async fn append(&self, meta: &CommandMeta, result: &CommandResult);
    async fn list(&self) -> Vec<CommandMeta>;
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
        self.entries.lock().unwrap().push(meta.clone());
    }

    async fn list(&self) -> Vec<CommandMeta> {
        self.entries.lock().unwrap().clone()
    }

    async fn clear(&self) {
        self.entries.lock().unwrap().clear();
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
    pub fn new(
        auth: AuthRegistry,
        handler: Arc<dyn CommandHandler<C>>,
        history: Arc<dyn CommandHistory>,
    ) -> Self {
        Self { auth, handler, history, audit: None }
    }

    pub fn with_audit(mut self, audit: Arc<dyn AuditSink>) -> Self {
        self.audit = Some(audit);
        self
    }

    pub async fn execute(&self, env: CommandEnvelope<C>) -> Result<CommandResult, CommandError> {
        let ctx = self.auth.authenticate(&env)?;
        let res = self.handler.handle(env.clone(), ctx.clone()).await?;
        self.history.append(&env.meta, &res).await;
        if let Some(sink) = &self.audit {
            let status = match &res {
                CommandResult::Error(e) => e.clone(),
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
    Set { key: String, value: String },
    Get { key: String },
    List,
    Reset,
    ReadConfig { path: String },
    WriteConfig { path: String, value: String },
}

pub trait CommandLabel {
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
        }
    }
}

/// Registry of live config bindings (Adaptive values).
pub struct ConfigRegistry {
    entries: HashMap<String, Box<dyn ConfigEntry>>,
}

impl ConfigRegistry {
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

    pub fn write(&self, path: &str, raw: &str) -> Result<(), String> {
        let entry = self.entries.get(path).ok_or_else(|| format!("unknown config path: {path}"))?;
        entry.write(raw)
    }

    pub fn read(&self, path: &str) -> Result<String, String> {
        let entry = self.entries.get(path).ok_or_else(|| format!("unknown config path: {path}"))?;
        entry.read()
    }

    pub fn contains(&self, path: &str) -> bool {
        self.entries.contains_key(path)
    }
}

trait ConfigEntry: Send + Sync {
    fn write(&self, raw: &str) -> Result<(), String>;
    fn read(&self) -> Result<String, String>;
}

struct GenericConfig<T> {
    handle: crate::adaptive::Adaptive<T>,
    parse: Arc<dyn Fn(&str) -> Result<T, String> + Send + Sync>,
    render: Arc<dyn Fn(&T) -> String + Send + Sync>,
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

#[derive(Default)]
pub struct BuiltInHandler {
    store: Arc<Mutex<HashMap<String, String>>>,
    config_registry: Option<ConfigRegistry>,
}

impl BuiltInHandler {
    pub fn with_config_registry(mut self, registry: ConfigRegistry) -> Self {
        self.config_registry = Some(registry);
        self
    }

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
                let keys = self.store.lock().unwrap().keys().cloned().collect();
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
}
