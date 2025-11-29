use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::sync::OnceLock;

use jsonschema::JSONSchema;
use serde_json::json;

use super::{AuthPayload, CommandContext, CommandEnvelope, CommandLabel};

fn default_args() -> serde_json::Value {
    serde_json::Value::Object(serde_json::Map::new())
}

/// Canonical wire envelope exchanged by control-plane transports.
///
/// This is transport-agnostic: HTTP, gRPC, JSONL, etc. should all map to this
/// shape before entering the router.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TransportEnvelope {
    /// Stable command identifier (unique per request).
    pub id: String,
    /// Command label/name (e.g., "write_config").
    pub cmd: String,
    /// Arbitrary JSON args for the command.
    #[serde(default = "default_args")]
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
        // Panic is intentional: schema is bundled at build time and validated in CI.
        let raw = include_str!("../../schemas/transport-envelope.schema.json");
        let value: JsonValue = serde_json::from_str(raw).expect("valid transport-envelope schema");
        JSONSchema::compile(&value).expect("transport-envelope schema compiles")
    })
}

fn command_result_schema() -> &'static JSONSchema {
    static SCHEMA: OnceLock<JSONSchema> = OnceLock::new();
    SCHEMA.get_or_init(|| {
        // Panic is intentional: schema is bundled at build time and validated in CI.
        let raw = include_str!("../../schemas/command-result.schema.json");
        let value: JsonValue = serde_json::from_str(raw).expect("valid command-result schema");
        JSONSchema::compile(&value).expect("command-result schema compiles")
    })
}

#[cfg(debug_assertions)]
fn validate_envelope(env: &TransportEnvelope) -> Result<(), String> {
    let env_val = serde_json::to_value(env).map_err(|e| e.to_string())?;
    validate(transport_envelope_schema(), &env_val)
}

#[cfg(not(debug_assertions))]
fn validate_envelope(_env: &TransportEnvelope) -> Result<(), String> {
    Ok(())
}

#[cfg(debug_assertions)]
fn validate_result(res: &super::CommandResult) -> Result<(), String> {
    let res_val = command_result_to_schema_value(res);
    validate(command_result_schema(), &res_val)
}

#[cfg(not(debug_assertions))]
fn validate_result(_res: &super::CommandResult) -> Result<(), String> {
    Ok(())
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
    fn map_error(err: &Self::Error) -> String;
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
    pub const MAX_REQUEST_SIZE: usize = 1024 * 1024; // 1 MiB

    /// Create a new TransportRouter.
    pub fn new(router: crate::control::CommandRouter<C>, transport: T, to_command: Conv) -> Self {
        Self { router, transport, to_command }
    }

    pub async fn handle(&self, raw: &[u8]) -> Result<Vec<u8>, String> {
        if raw.len() > Self::MAX_REQUEST_SIZE {
            return Err("request exceeds maximum size".into());
        }
        let env = self.transport.decode(raw).map_err(|e| T::map_error(&e))?;

        // Runtime validation of incoming envelope against JSON Schema (gated for perf)
        validate_envelope(&env)?;

        let (cmd_env, ctx) = (self.to_command)(env)?;
        let res = self.router.execute(cmd_env).await.map_err(|e| e.to_string())?;

        // Runtime validation of outgoing CommandResult against JSON Schema (gated for perf)
        validate_result(&res)?;

        self.transport.encode(&ctx, &res).map_err(|e| T::map_error(&e))
    }
}
