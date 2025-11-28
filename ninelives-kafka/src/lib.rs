//! Kafka telemetry sink for `ninelives` (companion crate).
//! Default build is a no-op to keep the core light; enable `client` to emit to Kafka.

use ninelives::telemetry::{PolicyEvent, TelemetrySink};
use serde_json::json;
use std::convert::Infallible;
use std::pin::Pin;
use std::task::{Context, Poll};

#[derive(Clone, Debug)]
pub struct KafkaSink {
    topic: String,
    producer: rdkafka::producer::FutureProducer,
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
        #[cfg(feature = "client")]
        let fut = {
            use rdkafka::producer::FutureRecord;
            let topic = self.topic.clone();
            let producer = self.producer.clone();
            let payload =
                serde_json::to_vec(&event_to_json(&event)).unwrap_or_else(|_| b"{}".to_vec());
            Box::pin(async move {
                let _ = producer.send(FutureRecord::to(&topic).payload(&payload), 0).await;
                Ok(())
            })
        };

        #[cfg(not(feature = "client"))]
        let fut = {
            let _ = serde_json::to_vec(&event_to_json(&event));
            Box::pin(async move { Ok(()) })
        };

        fut
    }
}

impl TelemetrySink for KafkaSink {
    type SinkError = Infallible;
}

fn event_to_json(event: &PolicyEvent) -> serde_json::Value {
    use ninelives::telemetry::{
        BulkheadEvent, CircuitBreakerEvent, RequestOutcome, RetryEvent, TimeoutEvent,
    };
    match event {
        PolicyEvent::Retry(r) => match r {
            RetryEvent::Attempt { attempt, delay } => {
                json!({ "kind": "retry_attempt", "attempt": attempt, "delay_ms": delay.as_millis() })
            }
            RetryEvent::Exhausted { total_attempts, total_duration } => {
                json!({ "kind": "retry_exhausted", "attempts": total_attempts, "duration_ms": total_duration.as_millis() })
            }
        },
        PolicyEvent::CircuitBreaker(c) => match c {
            CircuitBreakerEvent::Opened { failure_count } => {
                json!({ "kind": "circuit_opened", "failures": failure_count })
            }
            CircuitBreakerEvent::HalfOpen => json!({ "kind": "circuit_half_open" }),
            CircuitBreakerEvent::Closed => json!({ "kind": "circuit_closed" }),
        },
        PolicyEvent::Bulkhead(b) => match b {
            BulkheadEvent::Acquired { active_count, max_concurrency } => {
                json!({ "kind": "bulkhead_acquired", "active": active_count, "max": max_concurrency })
            }
            BulkheadEvent::Rejected { active_count, max_concurrency } => {
                json!({ "kind": "bulkhead_rejected", "active": active_count, "max": max_concurrency })
            }
        },
        PolicyEvent::Timeout(t) => match t {
            TimeoutEvent::Occurred { timeout } => {
                json!({ "kind": "timeout", "timeout_ms": timeout.as_millis() })
            }
        },
        PolicyEvent::Request(r) => match r {
            RequestOutcome::Success { duration } => {
                json!({ "kind": "request_success", "duration_ms": duration.as_millis() })
            }
            RequestOutcome::Failure { duration } => {
                json!({ "kind": "request_failure", "duration_ms": duration.as_millis() })
            }
        },
    }
}
