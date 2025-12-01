#![cfg(feature = "etcd-client")]

//! etcd telemetry sink for `ninelives` (companion crate).
//! Bring your own `etcd_client::Client`; events are stored as JSON under a prefix.

use ninelives::telemetry::{event_to_json, PolicyEvent, TelemetrySink};
use std::convert::Infallible;
use std::pin::Pin;
use std::task::{Context, Poll};

#[derive(Clone)]
pub struct EtcdSink {
    prefix: String,
    client: etcd_client::Client,
}

impl std::fmt::Debug for EtcdSink {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EtcdSink")
            .field("prefix", &self.prefix)
            .field("client", &"<etcd_client::Client>")
            .finish()
    }
}

impl EtcdSink {
    /// Create a sink using an existing etcd client; keys will be `prefix/<nanos>`.
    ///
    /// # Errors
    /// Returns `Err` if the prefix is empty, contains control characters, or is otherwise invalid.
    pub fn new(prefix: impl Into<String>, client: etcd_client::Client) -> Result<Self, String> {
        let mut p: String = prefix.into();

        // Normalize: trim whitespace and strip trailing slashes
        p = p.trim().trim_end_matches('/').to_string();

        // Validate
        if p.is_empty() {
            return Err("prefix cannot be empty".to_string());
        }
        if p.chars().any(|c| c.is_control()) {
            return Err("prefix cannot contain control characters".to_string());
        }

        Ok(Self { prefix: p, client })
    }
}

impl tower_service::Service<PolicyEvent> for EtcdSink {
    type Response = ();
    type Error = Infallible;
    type Future = Pin<Box<dyn std::future::Future<Output = Result<(), Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, event: PolicyEvent) -> Self::Future {
        let mut client = self.client.clone();
        let ts = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(i64::MAX); // chrono overflows near year 2262; clamp to max
        let key = format!("{}/{}-{}", self.prefix, ts, uuid::Uuid::new_v4());
        let value = event_to_json(&event);
        Box::pin(async move {
            match client.put(key.clone(), value.to_string(), None).await {
                Ok(_) => {
                    // Success: could increment a success metric here
                }
                Err(e) => {
                    tracing::warn!(
                        target: "ninelives::etcd",
                        key=%key,
                        error=%e,
                        "failed to write telemetry event to etcd"
                    );
                    // Failure: could increment a failure metric here
                }
            }
            Ok(())
        })
    }
}

impl TelemetrySink for EtcdSink {
    type SinkError = Infallible;
}
