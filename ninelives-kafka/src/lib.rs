//! Kafka telemetry sink for `ninelives` (companion crate).
//! Default build is a no-op to keep the core light; enable `client` to emit to Kafka.

use ninelives::telemetry::{PolicyEvent, TelemetrySink};
use std::convert::Infallible;
use std::pin::Pin;
use std::task::{Context, Poll};

#[derive(Clone, Debug)]
pub struct KafkaSink {
    topic: String,
    #[cfg(feature = "client")]
    producer: rdkafka::producer::FutureProducer,
}

impl KafkaSink {
    pub fn new<S: Into<String>>(brokers: S, topic: S) -> Result<Self, Box<dyn std::error::Error>> {
        let topic = topic.into();
        #[cfg(feature = "client")]
        {
            use rdkafka::config::ClientConfig;
            let producer = ClientConfig::new().set("bootstrap.servers", brokers.into()).create()?;
            return Ok(Self { topic, producer });
        }
        #[cfg(not(feature = "client"))]
        {
            let _ = brokers; // silence unused
            Ok(Self { topic, ..Self::noop() })
        }
    }

    #[cfg(not(feature = "client"))]
    fn noop() -> Self {
        Self { topic: String::new() }
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
        #[cfg(feature = "client")]
        let fut = {
            use rdkafka::producer::FutureRecord;
            let topic = self.topic.clone();
            let producer = self.producer.clone();
            let payload = format!("{:?}", event).into_bytes();
            Box::pin(async move {
                let _ = producer.send(FutureRecord::to(&topic).payload(&payload), 0).await;
                Ok(())
            })
        };

        #[cfg(not(feature = "client"))]
        let fut = Box::pin(async move { Ok(()) });

        fut
    }
}

impl TelemetrySink for KafkaSink {
    type SinkError = Infallible;
}
