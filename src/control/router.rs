use super::auth::AuthRegistry;
use super::handler::CommandHandler;
use super::types::*;
use async_trait::async_trait;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use serde_json::Value as JsonValue;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;

/// Default capacity for InMemoryHistory.
pub const DEFAULT_HISTORY_CAPACITY: usize = 1_000;

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
    async fn list(&self) -> Vec<HistoryRecord>;
    /// Clear history.
    async fn clear(&self);
}

/// In-memory history (for tests / defaults).
#[derive(Clone)]
pub struct InMemoryHistory {
    entries: Arc<Mutex<std::collections::VecDeque<HistoryRecord>>>,
    capacity: usize,
}

impl Default for InMemoryHistory {
    fn default() -> Self {
        Self::new(DEFAULT_HISTORY_CAPACITY)
    }
}

impl InMemoryHistory {
    /// Create a new in-memory history with a specific capacity.
    pub fn new(capacity: usize) -> Self {
        Self {
            entries: Arc::new(Mutex::new(std::collections::VecDeque::new())),
            capacity: capacity.max(1), // Ensure capacity is at least 1
        }
    }

    /// Returns the configured capacity of the history.
    pub fn capacity(&self) -> usize {
        self.capacity
    }
}

#[async_trait]
impl CommandHistory for InMemoryHistory {
    async fn append(&self, meta: &CommandMeta, result: &CommandResult) {
        let mut guard = self.entries.lock().await;
        guard.push_back(HistoryRecord { meta: meta.clone(), result: result.clone() });
        if guard.len() > self.capacity {
            guard.pop_front();
        }
    }

    async fn list(&self) -> Vec<HistoryRecord> {
        self.entries.lock().await.iter().cloned().collect()
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

    /// Retrieve recorded audit records (testing/diagnostics only).
    ///
    /// # Warning
    /// Clones the entire history; avoid in hot paths.
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

impl<C> Clone for CommandRouter<C> {
    fn clone(&self) -> Self {
        Self {
            auth: self.auth.clone(),
            handler: self.handler.clone(),
            history: self.history.clone(),
            audit: self.audit.clone(),
        }
    }
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
        // Extract a best-effort principal even if auth ultimately fails.
        let fallback_principal = extract_principal(&env);
        // Auth path: audit denials too.
        let auth_result = self.auth.authenticate(&env);
        let ctx = match auth_result {
            Ok(ctx) => ctx,
            Err(e) => {
                if let Some(sink) = &self.audit {
                    let record = AuditRecord {
                        id: env.meta.id.clone(),
                        label: env.cmd.label().into(),
                        principal: fallback_principal,
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

/// Attempt to extract a caller identity string without failing hard.
fn extract_principal<C: Clone>(env: &CommandEnvelope<C>) -> String {
    if let Some(auth) = &env.auth {
        match auth {
            AuthPayload::Mtls { peer_dn, .. } => return peer_dn.clone(),
            AuthPayload::Signatures { signatures, .. } => {
                if let Some(sig) = signatures.first() {
                    if let Some(kid) = &sig.key_id {
                        return kid.clone();
                    }
                }
            }
            AuthPayload::Jwt { token } => {
                let parts: Vec<&str> = token.split('.').collect();
                if parts.len() == 3 {
                    if let Ok(payload) = URL_SAFE_NO_PAD.decode(parts[1]) {
                        if let Ok(json) = serde_json::from_slice::<JsonValue>(&payload) {
                            if let Some(sub) = json.get("sub").and_then(|v| v.as_str()) {
                                return sub.to_string();
                            }
                        }
                    }
                }
            }
            AuthPayload::Opaque(_) => {}
        }
    }

    // Fall back to any identity hint in metadata (e.g., correlation id) or "unknown".
    env.meta.correlation_id.clone().unwrap_or_else(|| "unknown".to_string())
}
