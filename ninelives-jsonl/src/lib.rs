//! JSONL sink for `ninelives`. Writes one event per line.
//! Always writes; bring your own path.

use ninelives::telemetry::{event_to_json, PolicyEvent, TelemetrySink};
use serde_json::json;
use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};

#[derive(Clone, Debug)]
pub struct JsonlSink {
    path: std::path::PathBuf,
    file: std::sync::Arc<tokio::sync::Mutex<Option<tokio::fs::File>>>,
}

impl JsonlSink {
    pub fn new<P: Into<std::path::PathBuf>>(path: P) -> Self {
        Self { path: path.into(), file: std::sync::Arc::new(tokio::sync::Mutex::new(None)) }
    }
}

impl tower_service::Service<PolicyEvent> for JsonlSink {
    type Response = ();
    type Error = io::Error;
    type Future = Pin<Box<dyn std::future::Future<Output = Result<(), Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, event: PolicyEvent) -> Self::Future {
        let path = self.path.clone();
        let file = self.file.clone();
        let line = json!(event_to_json(&event)).to_string() + "\n";
        Box::pin(async move {
            use tokio::io::AsyncWriteExt;
            let mut guard = file.lock().await;
            if guard.is_none() {
                let f = tokio::fs::OpenOptions::new().create(true).append(true).open(path).await?;
                *guard = Some(f);
            }
            if let Some(f) = guard.as_mut() {
                f.write_all(line.as_bytes()).await?;
                f.flush().await?;
            }
            Ok(())
        })
    }
}

impl TelemetrySink for JsonlSink {
    type SinkError = io::Error;
}
