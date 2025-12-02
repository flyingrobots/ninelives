use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;

/// Opaque command identifier.
pub type CommandId = String;

/// Execution metadata attached to each command.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct CommandMeta {
    /// Command identifier (unique per request).
    pub id: CommandId,
    /// Optional correlation ID for tracing.
    pub correlation_id: Option<String>,
    /// Timestamp in milliseconds (epoch).
    pub timestamp_millis: Option<u128>,
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

/// Auth payload sent alongside a command. Transports set this; providers verify it.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[non_exhaustive]
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

/// Command envelope carrying the command, auth payload, and metadata.
#[derive(Clone, Debug)]
pub struct CommandEnvelope {
    /// The command payload.
    pub cmd: Box<dyn super::command::Command>,
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
#[non_exhaustive]
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

/// Errors returned by command handling.
#[derive(thiserror::Error, Debug)]
#[non_exhaustive]
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
    /// Missing circuit breaker registry when requested by a command.
    #[error("circuit breaker registry missing: {hint}")]
    BreakerRegistryMissing {
        /// Guidance on how to supply the registry.
        hint: &'static str,
    },
    /// Audit recording failed.
    #[error("audit: {0}")]
    Audit(String),
}

/// Structured command failure payload.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[non_exhaustive]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CommandFailure {
    /// Caller provided invalid arguments.
    InvalidArgs {
        /// human-friendly description of the invalid input
        msg: String,
    },
    /// Requested resource was not found.
    NotFound {
        /// identifier of missing resource
        what: String,
    },
    /// Required registry dependency missing.
    RegistryMissing {
        /// how to supply the missing registry
        hint: String,
    },
    /// Catch-all internal error.
    Internal {
        /// internal error message
        msg: String,
    },
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
#[non_exhaustive]
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

/// Record of a command execution for history.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct HistoryRecord {
    /// Metadata from the request.
    pub meta: CommandMeta,
    /// Result of the execution.
    pub result: CommandResult,
}

