//! Control plane primitives: command envelope, auth, history, router.
//!
//! This is a lightweight, transport-agnostic control plane. Transports populate
//! `CommandEnvelope` with an `AuthPayload`; the router dispatches to handlers
//! after auth. History storage is pluggable.

/// Authentication providers and payload verification.
pub mod auth;
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

// Re-export everything for convenience and backward compatibility.
pub use auth::*;
pub use handler::*;
pub use router::*;
pub use types::*;
