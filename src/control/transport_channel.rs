use std::sync::Arc;

use tokio::sync::{mpsc, oneshot};

use super::{CommandEnvelope, CommandLabel, CommandResult};
use crate::control::CommandRouter;

/// In-process channel-based transport for the control plane.
type Tx<C> = mpsc::Sender<(CommandEnvelope<C>, oneshot::Sender<Result<CommandResult, String>>)>;

pub struct ChannelTransport<C: Clone> {
    tx: Tx<C>,
}

impl<C> ChannelTransport<C>
where
    C: CommandLabel + Clone + Send + Sync + 'static,
{
    /// Create a channel transport and spawn a worker to drive the router.
    pub fn new(router: Arc<CommandRouter<C>>) -> Self {
        let (tx, mut rx) = mpsc::channel::<(
            CommandEnvelope<C>,
            oneshot::Sender<Result<CommandResult, String>>,
        )>(64);
        tokio::spawn(async move {
            while let Some((env, reply_tx)) = rx.recv().await {
                let res = router.execute(env).await.map_err(|e| format!("{}", e));
                let _ = reply_tx.send(res);
            }
        });
        Self { tx }
    }

    /// Send a command and await the result over the channel.
    pub async fn send(&self, env: CommandEnvelope<C>) -> Result<CommandResult, String> {
        let (resp_tx, resp_rx) = oneshot::channel();
        self.tx.send((env, resp_tx)).await.map_err(|e| e.to_string())?;
        resp_rx.await.map_err(|e| e.to_string())?
    }
}
