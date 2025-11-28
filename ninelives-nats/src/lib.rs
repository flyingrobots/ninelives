//! NATS telemetry sink for `ninelives` (companion crate).
//!
//! Bring your own async `async_nats::Client`; events are serialized to
//! JSON and published to the configured subject.
//!
//! ```rust
//! use ninelives_nats::NatsSink;
//! # use ninelives::telemetry::PolicyEvent;
//! # async fn demo() -> Result<(), Box<dyn std::error::Error>> {
//! let client = async_nats::connect("nats://127.0.0.1:4222").await?;
//! let sink = NatsSink::new(client, "policy.events");
//! // wrap with NonBlockingSink if desired
//! # Ok(()) }
//! ```

use ninelives::telemetry::{PolicyEvent, TelemetrySink};
use serde_json::json;
use std::convert::Infallible;
use std::pin::Pin;
use std::task::{Context, Poll};

#[derive(Clone, Debug)]
pub struct NatsSink {
    subject: String,
    client: async_nats::Client,
}

impl NatsSink {
    /// Create a sink using an existing NATS async connection.
    pub fn new(client: async_nats::Client, subject: impl Into<String>) -> Self {
        Self { subject: subject.into(), client }
    }
}

impl tower_service::Service<PolicyEvent> for NatsSink {
    type Response = ();
    type Error = Infallible;
    type Future = Pin<Box<dyn std::future::Future<Output = Result<(), Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, event: PolicyEvent) -> Self::Future {
        let subject = self.subject.clone();
        let client = self.client.clone();
        let payload = serde_json::to_vec(&event_to_json(&event)).unwrap_or_else(|_| b"{}".to_vec());
        Box::pin(async move {
            let _ = client.publish(subject, payload.into()).await;
            Ok(())
        })
    }
}

impl TelemetrySink for NatsSink {
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
