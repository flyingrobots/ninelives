use std::sync::Arc;

use tokio::sync::{mpsc, oneshot};

use super::{CommandEnvelope, CommandLabel, CommandResult};
use crate::control::CommandRouter;

/// Errors that can occur during transport operations.
#[derive(Debug, thiserror::Error)]
pub enum TransportError {
    /// The channel to the router worker is closed.
    #[error("transport worker channel closed")]
    ChannelClosed,
    /// The response channel was closed before a response was received.
    #[error("response channel closed (worker died or dropped sender)")]
    ResponseLost,
    /// The router returned an error.
    #[error("router error: {0}")]
    RouterError(String),
}

/// In-process channel-based transport for the control plane.
type Tx<C> = mpsc::Sender<(CommandEnvelope<C>, oneshot::Sender<Result<CommandResult, TransportError>>)>;

/// A transport that sends commands over a Tokio MPSC channel.
/// useful for in-process communication or testing.
pub struct ChannelTransport<C: Clone> {
    tx: Tx<C>,
}

impl<C> ChannelTransport<C>
where
    C: CommandLabel + Clone + Send + Sync + 'static,
{
    /// Create a channel transport with default capacity (64).
    pub fn new(router: Arc<CommandRouter<C>>) -> Self {
        Self::with_capacity(router, 64)
    }

    /// Create a channel transport with specified capacity.
    pub fn with_capacity(router: Arc<CommandRouter<C>>, capacity: usize) -> Self {
        let (tx, mut rx) = mpsc::channel::<(
            CommandEnvelope<C>,
            oneshot::Sender<Result<CommandResult, TransportError>>,
        )>(capacity);
        tokio::spawn(async move {
            while let Some((env, reply_tx)) = rx.recv().await {
                let res = router
                    .execute(env)
                    .await
                    .map_err(|e| TransportError::RouterError(format!("{:?}", e)));
                let _ = reply_tx.send(res);
            }
        });
        Self { tx }
    }

    /// Send a command and await the result over the channel.
    pub async fn send(&self, env: CommandEnvelope<C>) -> Result<CommandResult, TransportError> {
        let (resp_tx, resp_rx) = oneshot::channel();
        self.tx.send((env, resp_tx)).await.map_err(|_| TransportError::ChannelClosed)?;
        resp_rx.await.map_err(|_| TransportError::ResponseLost)?
    }
}
