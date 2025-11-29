//! Kafka telemetry sink for `ninelives` (companion crate).
//! Bring your own `FutureProducer`; events are sent as JSON payloads.

use ninelives::telemetry::{event_to_json, PolicyEvent, TelemetrySink};
use std::convert::Infallible;
use std::pin::Pin;
use std::task::{Context, Poll};

#[derive(Clone)]
pub struct KafkaSink {
    topic: String,
    producer: rdkafka::producer::FutureProducer,
}

impl std::fmt::Debug for KafkaSink {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KafkaSink").field("topic", &self.topic).finish()
    }
}

impl KafkaSink {
    /// Create a sink with an existing Kafka producer.
    pub fn new(producer: rdkafka::producer::FutureProducer, topic: impl Into<String>) -> Self {
        Self { topic: topic.into(), producer }
    }
}

impl tower_service::Service<PolicyEvent> for KafkaSink {
    type Response = ();
    type Error = Infallible;
    type Future = Pin<Box<dyn std::future::Future<Output = Result<(), Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, event: PolicyEvent) -> Self::Future {
        use rdkafka::producer::FutureRecord;
        let topic = self.topic.clone();
        let producer = self.producer.clone();
        let payload = match serde_json::to_vec(&event_to_json(&event)) {
            Ok(p) => p,
            Err(e) => {
                tracing::warn!("KafkaSink: failed to serialize event: {e}");
                return Box::pin(async { Ok(()) });
            }
        };
        Box::pin(async move {
            let record = FutureRecord::<(), _>::to(&topic).payload(&payload);
            if let Err((e, _)) = producer.send(record, None).await {
                tracing::warn!("KafkaSink: failed to send event: {e}");
            }
            Ok(())
        })
    }
}

impl TelemetrySink for KafkaSink {
    type SinkError = Infallible;
}
