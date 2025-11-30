use super::auth::AuthRegistry;
use super::handler::CommandHandler;
use super::types::*;
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;

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
        Self { entries: Arc::new(Mutex::new(std::collections::VecDeque::new())), capacity: 1_000 }
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
