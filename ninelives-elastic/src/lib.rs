//! Elasticsearch telemetry sink for `ninelives`.
//! Default build is a no-op; enable the `client` feature to index events.

use ninelives::telemetry::{PolicyEvent, TelemetrySink};
use std::convert::Infallible;
use std::pin::Pin;
use std::task::{Context, Poll};

#[derive(Clone, Debug)]
pub struct ElasticSink {
    index: String,
    #[cfg(feature = "client")]
    client: elasticsearch::Elasticsearch,
}

impl ElasticSink {
    pub fn new<S: Into<String>>(endpoint: S, index: S) -> Result<Self, Box<dyn std::error::Error>> {
        let index = index.into();
        #[cfg(feature = "client")]
        {
            let transport =
                elasticsearch::http::transport::Transport::single_node(&endpoint.into())?;
            let client = elasticsearch::Elasticsearch::new(transport);
            return Ok(Self { index, client });
        }
        #[cfg(not(feature = "client"))]
        {
            let _ = endpoint;
            Ok(Self { index, ..Self::noop() })
        }
    }

    #[cfg(not(feature = "client"))]
    fn noop() -> Self {
        Self { index: String::new() }
    }
}

impl tower_service::Service<PolicyEvent> for ElasticSink {
    type Response = ();
    type Error = Infallible;
    type Future = Pin<Box<dyn std::future::Future<Output = Result<(), Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, event: PolicyEvent) -> Self::Future {
        #[cfg(feature = "client")]
        let fut = {
            use elasticsearch::http::request::JsonBody;
            use elasticsearch::indices::IndicesCreateParts;
            use elasticsearch::IndexParts;

            let client = self.client.clone();
            let index = self.index.clone();
            Box::pin(async move {
                // ensure index exists (best-effort)
                let _ = client.indices().create(IndicesCreateParts::Index(&index)).send().await;

                let body = JsonBody::new(format!("{{\"event\":\"{:?}\"}}", event));
                let _ = client.index(IndexParts::Index(&index)).body(body).send().await;
                Ok(())
            })
        };

        #[cfg(not(feature = "client"))]
        let fut = Box::pin(async move { Ok(()) });

        fut
    }
}

impl TelemetrySink for ElasticSink {
    type SinkError = Infallible;
}
