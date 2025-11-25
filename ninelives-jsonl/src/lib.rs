//! JSONL sink for `ninelives`. Writes one event per line.
//! Default build is no-op. Enable `async-fs` to write with tokio fs.

use ninelives::telemetry::{PolicyEvent, TelemetrySink};
use std::convert::Infallible;
use std::pin::Pin;
use std::task::{Context, Poll};

#[derive(Clone, Debug)]
pub struct JsonlSink {
    path: String,
}

impl JsonlSink {
    pub fn new<S: Into<String>>(path: S) -> Self {
        Self { path: path.into() }
    }
}

impl tower_service::Service<PolicyEvent> for JsonlSink {
    type Response = ();
    type Error = Infallible;
    type Future = Pin<Box<dyn std::future::Future<Output = Result<(), Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, event: PolicyEvent) -> Self::Future {
        #[cfg(feature = "async-fs")]
        {
            let path = self.path.clone();
            let line = serde_json::json!({ "event": format!("{:?}", event) }).to_string() + "\n";
            return Box::pin(async move {
                use tokio::io::AsyncWriteExt;
                let mut file = tokio::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(path)
                    .await
                    .map_err(|_| Infallible)?;
                let _ = file.write_all(line.as_bytes()).await;
                Ok(())
            });
        }
        #[cfg(not(feature = "async-fs"))]
        {
            return Box::pin(async move { Ok(()) });
        }
    }
}

impl TelemetrySink for JsonlSink {
    type SinkError = Infallible;
}
