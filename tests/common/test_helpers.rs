#[cfg(feature = "control")]
use ninelives::control::{AuthPayload, BuiltInCommand, CommandEnvelope, CommandMeta};

#[cfg(feature = "control")]
pub fn create_test_envelope(
    cmd: BuiltInCommand,
    id: Option<&str>,
    correlation_id: Option<&str>,
    auth: Option<AuthPayload>,
    timestamp: Option<u128>,
) -> CommandEnvelope<BuiltInCommand> {
    CommandEnvelope {
        cmd,
        auth: auth.or_else(|| Some(AuthPayload::Opaque(vec![]))), // Default auth
        meta: CommandMeta {
            id: id.unwrap_or("test-id").to_string(),
            correlation_id: correlation_id.map(|s| s.to_string()),
            timestamp_millis: timestamp,
        },
    }
}
