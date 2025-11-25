//! Control plane primitives: command envelope, auth, history, router.
//!
//! This is a lightweight, transport-agnostic nucleus. Transports populate
//! `CommandEnvelope` with an `AuthPayload`; the router dispatches to handlers
//! after auth. History storage is pluggable.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;

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
    Signatures {
        payload_hash: [u8; 32],
        signatures: Vec<DetachedSig>,
    },
    Mtls {
        peer_dn: String,
        cert_chain: Vec<Vec<u8>>,
    },
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
    fn authenticate(&self, meta: &CommandMeta, auth: Option<&AuthPayload>) -> Result<AuthContext, AuthError>;

    /// Optional authorization using the command label.
    fn authorize(&self, _ctx: &AuthContext, _label: &str, _meta: &CommandMeta) -> Result<(), AuthError> {
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
    fn name(&self) -> &'static str { "passthrough" }
    fn authenticate(&self, _meta: &CommandMeta, _auth: Option<&AuthPayload>) -> Result<AuthContext, AuthError> {
        Ok(AuthContext { principal: "anonymous".into(), provider: self.name(), attributes: HashMap::new() })
    }
}

/// Command handler trait.
#[async_trait]
pub trait CommandHandler<C: Clone>: Send + Sync {
    async fn handle(&self, cmd: CommandEnvelope<C>, ctx: AuthContext) -> Result<CommandResult, CommandError>;
}

#[derive(thiserror::Error, Debug)]
pub enum CommandError {
    #[error("auth: {0}")]
    Auth(#[from] AuthError),
    #[error("handler: {0}")]
    Handler(String),
}

#[derive(Clone, Debug, PartialEq)]
pub enum CommandResult {
    Ack,
    Value(String),
    List(Vec<String>),
    Reset,
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
}

impl<C> CommandRouter<C>
where
    C: Send + Sync + Clone + CommandLabel + 'static,
{
    pub fn new(auth: AuthRegistry, handler: Arc<dyn CommandHandler<C>>, history: Arc<dyn CommandHistory>) -> Self {
        Self { auth, handler, history }
    }

    pub async fn execute(&self, env: CommandEnvelope<C>) -> Result<CommandResult, CommandError> {
        let ctx = self.auth.authenticate(&env)?;
        let res = self.handler.handle(env.clone(), ctx).await?;
        self.history.append(&env.meta, &res).await;
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
        }
    }
}

#[derive(Default)]
pub struct BuiltInHandler {
    store: Arc<Mutex<HashMap<String, String>>>,
}

#[async_trait]
impl CommandHandler<BuiltInCommand> for BuiltInHandler {
    async fn handle(&self, cmd: CommandEnvelope<BuiltInCommand>, _ctx: AuthContext) -> Result<CommandResult, CommandError> {
        match cmd.cmd {
            BuiltInCommand::Set { key, value } => {
                self.store.lock().unwrap().insert(key, value);
                Ok(CommandResult::Ack)
            }
            BuiltInCommand::Get { key } => {
                let val = self.store.lock().unwrap().get(&key).cloned().unwrap_or_default();
                Ok(CommandResult::Value(val))
            }
            BuiltInCommand::List => {
                let keys = self.store.lock().unwrap().keys().cloned().collect();
                Ok(CommandResult::List(keys))
            }
            BuiltInCommand::Reset => {
                self.store.lock().unwrap().clear();
                Ok(CommandResult::Reset)
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

        let res = router.execute(env(BuiltInCommand::Set { key: "k".into(), value: "v".into() })).await.unwrap();
        assert_eq!(res, CommandResult::Ack);

        let res = router.execute(env(BuiltInCommand::Get { key: "k".into() })).await.unwrap();
        assert_eq!(res, CommandResult::Value("v".into()));

        let hist = history.list().await;
        assert_eq!(hist.len(), 2);
    }

    struct DenyAll;
    impl AuthProvider for DenyAll {
        fn name(&self) -> &'static str { "deny" }
        fn authenticate(&self, _meta: &CommandMeta, _auth: Option<&AuthPayload>) -> Result<AuthContext, AuthError> {
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
}
