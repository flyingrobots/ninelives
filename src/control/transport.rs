use serde::{Deserialize, Serialize};

use super::{AuthPayload, CommandContext, CommandEnvelope, CommandLabel};

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

/// Bridges raw transport frames to a CommandRouter using a decoder/encoder pair.
pub struct TransportRouter<C, T, Conv>
where
    C: CommandLabel + Clone + Send + Sync + 'static,
    T: Transport,
    Conv:
        Fn(TransportEnvelope) -> Result<(CommandEnvelope<C>, CommandContext), String> + Send + Sync,
{
    router: crate::control::CommandRouter<C>,
    transport: T,
    to_command: Conv,
}

impl<C, T, Conv> TransportRouter<C, T, Conv>
where
    C: CommandLabel + Clone + Send + Sync + 'static,
    T: Transport,
    Conv:
        Fn(TransportEnvelope) -> Result<(CommandEnvelope<C>, CommandContext), String> + Send + Sync,
{
    pub fn new(router: crate::control::CommandRouter<C>, transport: T, conv: Conv) -> Self {
        Self { router, transport, to_command: conv }
    }

    /// Handle a raw request frame and return encoded response bytes.
    pub async fn handle(&self, raw: &[u8]) -> Result<Vec<u8>, String> {
        let env = self.transport.decode(raw).map_err(T::map_error)?;
        let (cmd_env, ctx) = (self.to_command)(env)?;
        let res = self.router.execute(cmd_env).await.map_err(|e| format!("{}", e))?;
        self.transport.encode(&ctx, &res).map_err(T::map_error)
    }
}
