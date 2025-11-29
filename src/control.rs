//! Control plane primitives: command envelope, auth, history, router.
//!
//! This is a lightweight, transport-agnostic control plane. Transports populate
//! `CommandEnvelope` with an `AuthPayload`; the router dispatches to handlers
//! after auth. History storage is pluggable.

pub mod auth;
pub mod handler;
pub mod router;
pub mod transport;
pub mod transport_channel;
pub mod types;

// Re-export everything for convenience and backward compatibility.
pub use auth::*;
pub use handler::*;
pub use router::*;
pub use types::*;
