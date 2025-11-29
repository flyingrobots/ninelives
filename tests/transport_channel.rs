#![allow(missing_docs)]

use ninelives::control::{
    AuthMode, AuthPayload, AuthRegistry, BuiltInCommand, CommandEnvelope, CommandMeta,
    CommandResult, InMemoryHistory, PassthroughAuth,
};
use ninelives::ChannelTransport;
use std::sync::Arc;
use std::time::Duration;

use crate::common::test_helpers;

fn env(cmd: BuiltInCommand) -> CommandEnvelope<BuiltInCommand> {
    test_helpers::create_test_envelope(cmd, Some("chan-1"), None, None, None)
}

#[tokio::test]
async fn channel_transport_roundtrip() {
    let mut auth = AuthRegistry::new(AuthMode::First);
    auth.register(Arc::new(PassthroughAuth));
    let handler = Arc::new(ninelives::control::BuiltInHandler::default());
    let history: Arc<dyn ninelives::control::CommandHistory> = Arc::new(InMemoryHistory::default());
    let router = Arc::new(ninelives::control::CommandRouter::new(auth, handler, history));

    let transport = ChannelTransport::new(router);
    let res =
        tokio::time::timeout(Duration::from_secs(5), transport.send(env(BuiltInCommand::List)))
            .await
            .expect("transport.send timed out")
            .unwrap();
    assert_eq!(res, CommandResult::List(vec![]));
}
