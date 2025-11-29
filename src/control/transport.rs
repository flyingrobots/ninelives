use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::sync::OnceLock;

use jsonschema::JSONSchema;
use serde_json::json;

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

// -----------------------------------------------------------------------------
// Schema validation helpers (runtime enforced in TransportRouter)
// -----------------------------------------------------------------------------

fn transport_envelope_schema() -> &'static JSONSchema {
    static SCHEMA: OnceLock<JSONSchema> = OnceLock::new();
    SCHEMA.get_or_init(|| {
        let raw = include_str!("../../schemas/transport-envelope.schema.json");
        let value: JsonValue = serde_json::from_str(raw).expect("valid transport-envelope schema");
        JSONSchema::compile(&value).expect("transport-envelope schema compiles")
    })
}

fn command_result_schema() -> &'static JSONSchema {
    static SCHEMA: OnceLock<JSONSchema> = OnceLock::new();
    SCHEMA.get_or_init(|| {
        let raw = include_str!("../../schemas/command-result.schema.json");
        let value: JsonValue = serde_json::from_str(raw).expect("valid command-result schema");
        JSONSchema::compile(&value).expect("command-result schema compiles")
    })
}

fn validate(schema: &JSONSchema, value: &JsonValue) -> Result<(), String> {
    schema
        .validate(value)
        .map_err(|errs| errs.map(|e| e.to_string()).collect::<Vec<_>>().join(", "))
}

fn command_result_to_schema_value(res: &super::CommandResult) -> JsonValue {
    match res {
        super::CommandResult::Ack => json!({ "result": "ack" }),
        super::CommandResult::Value(v) => json!({ "result": "value", "value": v }),
        super::CommandResult::List(items) => json!({ "result": "list", "items": items }),
        super::CommandResult::Reset => json!({ "result": "reset" }),
        super::CommandResult::Error(msg) => json!({ "result": "error", "message": msg }),
    }
}

/// Transport abstraction for encoding/decoding control-plane messages.
pub trait Transport: Send + Sync {
    /// Error type produced by encoding/decoding operations.
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
    /// Create a new `TransportRouter` connecting a transport to a command router via a converter.
    pub fn new(router: crate::control::CommandRouter<C>, transport: T, conv: Conv) -> Self {
        Self { router, transport, to_command: conv }
    }

    /// Handle a raw request frame and return encoded response bytes.
    pub async fn handle(&self, raw: &[u8]) -> Result<Vec<u8>, String> {
        let env = self.transport.decode(raw).map_err(T::map_error)?;

        // Runtime validation of incoming envelope against JSON Schema
        let env_val = serde_json::to_value(&env).map_err(|e| e.to_string())?;
        validate(transport_envelope_schema(), &env_val)?;

        let (cmd_env, ctx) = (self.to_command)(env)?;
        let res = self.router.execute(cmd_env).await.map_err(|e| format!("{}", e))?;

        // Runtime validation of outgoing CommandResult against JSON Schema
        let res_val = command_result_to_schema_value(&res);
        validate(command_result_schema(), &res_val)?;

        self.transport.encode(&ctx, &res).map_err(T::map_error)
    }
}
