use serde::{Deserialize, Serialize};

use super::{AuthPayload, CommandContext};

/// Canonical wire envelope exchanged by control-plane transports.
///
/// This is transport-agnostic: HTTP, gRPC, JSONL, etc. should all map to this
/// shape before entering the router.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TransportEnvelope {
    /// Stable command identifier (unique per request).
    pub id: String,
    /// Command label/name (e.g., \"write_config\").
    pub cmd: String,
    /// Arbitrary JSON args for the command.
    #[serde(default)]
    pub args: serde_json::Value,
    /// Optional auth payload.
    #[serde(default)]
    pub auth: Option<AuthPayload>,
}

/// Transport abstraction for encoding/decoding control-plane messages.
pub trait Transport: Send + Sync {
    type Error: std::error::Error + Send + Sync + 'static;

    /// Decode a raw frame (e.g., bytes/string/json value) into a transport envelope.
    fn decode(&self, raw: &[u8]) -> Result<TransportEnvelope, Self::Error>;

    /// Encode a CommandContext + result into an outgoing frame.
    fn encode(
        &self,
        ctx: &CommandContext,
        result: &super::CommandResult,
    ) -> Result<Vec<u8>, Self::Error>;

    /// Map transport-specific errors into router-visible strings.
    fn map_error(err: Self::Error) -> String;
}
