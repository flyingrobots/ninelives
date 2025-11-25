//! etcd telemetry sink for `ninelives` (companion crate).
//! Default build is no-op; enable `client` to write events to etcd keys.

use ninelives::telemetry::{PolicyEvent, TelemetrySink};
use std::convert::Infallible;
use std::pin::Pin;
use std::task::{Context, Poll};

#[derive(Clone, Debug)]
pub struct EtcdSink {
    prefix: String,
    #[cfg(feature = "client")]
    client: etcd_client::Client,
}

impl EtcdSink {
    /// `endpoint` like "http://127.0.0.1:2379"; events stored under `prefix/<nanos>`
    pub async fn new<S: Into<String>>(endpoint: S, prefix: S) -> Result<Self, Box<dyn std::error::Error>> {
        let prefix = prefix.into();
        #[cfg(feature = "client")]
        {
            let client = etcd_client::Client::connect([endpoint.into()], None).await?;
            return Ok(Self { prefix, client });
        }
        #[cfg(not(feature = "client"))]
        {
            let _ = endpoint;
            Ok(Self { prefix, ..Self::noop() })
        }
    }

    #[cfg(not(feature = "client"))]
    fn noop() -> Self {
        Self { prefix: String::new() }
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
        #[cfg(feature = "client")]
        let fut = {
            let mut client = self.client.clone();
            let key = format!("{}/{}", self.prefix, chrono::Utc::now().timestamp_nanos());
            let val = format!("{:?}", event);
            Box::pin(async move {
                let _ = client.put(key, val, None).await;
                Ok(())
            })
        };

        #[cfg(not(feature = "client"))]
        let fut = Box::pin(async move { Ok(()) });

        fut
    }
}

impl TelemetrySink for EtcdSink {
    type SinkError = Infallible;
}
