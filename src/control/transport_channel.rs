use std::sync::Arc;

use tokio::sync::{mpsc, oneshot, watch};
use tokio::task::JoinHandle;

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
    shutdown_tx: watch::Sender<bool>,
    worker_handle: Arc<std::sync::Mutex<Option<JoinHandle<()>>>>,
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
        let (shutdown_tx, mut shutdown_rx) = watch::channel(false);

        let worker_handle = tokio::spawn(async move {
            tracing::info!("ChannelTransport worker started");
            loop {
                tokio::select! {
                    _ = shutdown_rx.changed() => {
                        tracing::info!("ChannelTransport received shutdown signal");
                        break;
                    }
                    msg = rx.recv() => {
                        match msg {
                            Some((env, reply_tx)) => {
                                let router = router.clone();
                                // Wrap processing in spawn_blocking or AssertUnwindSafe if panic recovery is critical.
                                // Here we use a standard tokio::spawn wrapper to isolate panics per request if feasible,
                                // but since we are processing sequentially, we catch unwind around the execution block.
                                let res = std::panic::AssertUnwindSafe(async {
                                    tracing::debug!(cmd_id=%env.meta.id, "Received command");
                                    router.execute(env).await
                                }).catch_unwind().await;

                                let result = match res {
                                    Ok(Ok(ok)) => Ok(ok),
                                    Ok(Err(e)) => {
                                        tracing::error!(error=%e, "Router execution failed");
                                        Err(TransportError::RouterError(format!("{:?}", e)))
                                    },
                                    Err(panic) => {
                                        tracing::error!(panic=?panic, "Router panicked during execution");
                                        Err(TransportError::RouterError("Internal panic".into()))
                                    }
                                };

                                if let Err(_) = reply_tx.send(result) {
                                    tracing::warn!("Failed to send response; receiver dropped");
                                }
                            }
                            None => {
                                tracing::info!("ChannelTransport input channel closed");
                                break;
                            }
                        }
                    }
                }
            }
            tracing::info!("ChannelTransport worker stopped");
        });

        Self {
            tx,
            shutdown_tx,
            worker_handle: Arc::new(std::sync::Mutex::new(Some(worker_handle))),
        }
    }

    /// Send a command and await the result over the channel.
    pub async fn send(&self, env: CommandEnvelope<C>) -> Result<CommandResult, TransportError> {
        let (resp_tx, resp_rx) = oneshot::channel();
        self.tx.send((env, resp_tx)).await.map_err(|_| TransportError::ChannelClosed)?;
        resp_rx.await.map_err(|_| TransportError::ResponseLost)?
    }

    /// Signal the worker to shut down and await its termination.
    pub async fn shutdown(&self) {
        let _ = self.shutdown_tx.send(true);
        let handle = {
            let mut lock = self.worker_handle.lock().unwrap();
            lock.take()
        };
        if let Some(h) = handle {
            let _ = h.await;
        }
    }
}

// Helper trait for catch_unwind
use futures::FutureExt;
