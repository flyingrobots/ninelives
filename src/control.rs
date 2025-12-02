//! Control plane primitives: command envelope, auth, history, router.
//!
//! This is a lightweight, transport-agnostic control plane. Transports populate
//! `CommandEnvelope` with an `AuthPayload`; the router dispatches to handlers
//! after auth. History storage is pluggable.

use std::sync::Arc;

use crate::ChannelTransport;

/// Authentication providers and payload verification.
pub mod auth;
/// Built-in command type definitions.
pub mod builtin_commands;
/// Factory for parsing built-in commands.
pub mod builtin_factory;
/// Command trait and registry for extensibility.
pub mod command;
/// Core command handler traits and built-in command definitions.
pub mod handler;
/// Command router orchestration (auth -> handler -> audit/history).
pub mod router;
/// Transport abstractions.
pub mod transport;
/// Channel-based transport implementation.
pub mod transport_channel;
/// Shared control-plane data types.
pub mod types;

// Re-exports from types
pub use types::{
    AuditRecord, AuthContext, AuthError, AuthPayload, CommandContext, CommandEnvelope,
    CommandError, CommandFailure, CommandId, CommandMeta, CommandResult, DetachedSig,
    HistoryRecord,
};

// Re-exports from auth
pub use auth::{
    AuthMode, AuthProvider, AuthRegistry, AuthorizationLayer, AuthorizationService, PassthroughAuth,
};

// Re-exports from command
pub use command::{Command, CommandFactory, CommandRegistry as CommandTypeRegistry};

// Re-exports from builtin_commands
pub use builtin_commands::{
    GetCommand, GetStateCommand, HealthCommand, ListCommand, ListConfigCommand,
    ReadConfigCommand, ResetCircuitBreakerCommand, ResetCommand, SetCommand, WriteConfigCommand,
};

// Re-exports from builtin_factory
pub use builtin_factory::BuiltInCommandFactory;

// Re-exports from handler
pub use handler::{
    BuiltInHandler, CommandHandler, CommandService, ConfigRegistry,
    DefaultConfigRegistry, InMemoryConfigRegistry,
};

// Re-exports from router
pub use router::{
    AuditSink, CommandHistory, CommandRouter, InMemoryHistory, MemoryAuditSink, TracingAuditSink,
};

// Note: transport and transport_channel are not re-exported from this module to avoid namespace
// pollution (`crate::control::transport::Transport`, etc.). However, the crate root intentionally
// re-exports specific transport types behind the `control` feature for convenience:
// `Transport`, `TransportEnvelope`, `TransportRouter`, and `ChannelTransport`.

/// Build a ready-to-use control plane with in-process transport and default registries.
///
/// This helper wires:
/// - `PassthroughAuth` (development only; replace for production)
/// - `DefaultConfigRegistry`
/// - `InMemoryHistory`
/// - `MemoryAuditSink`
/// - `ChannelTransport` targeting the router
pub fn bootstrap_defaults(
    handler: Arc<dyn handler::CommandHandler>,
) -> (Arc<CommandRouter>, ChannelTransport) {
    let mut auth = AuthRegistry::new(AuthMode::First);
    auth.register(Arc::new(PassthroughAuth));

    let _cfg = DefaultConfigRegistry::new();
    let history: Arc<dyn CommandHistory> = Arc::new(InMemoryHistory::default());
    use crate::control::router::DEFAULT_HISTORY_CAPACITY;
    let audit = Arc::new(MemoryAuditSink::new(DEFAULT_HISTORY_CAPACITY));

    let router = Arc::new(CommandRouter::new(auth, handler, history).with_audit(audit));
    (router.clone(), ChannelTransport::new(router))
}
