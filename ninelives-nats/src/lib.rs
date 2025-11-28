//! NATS telemetry sink for `ninelives` (optional companion crate).
//!
//! Default build is a no-op sink to keep dependencies light. Enable the `client`
//! feature to publish `PolicyEvent`s to a NATS subject.
//!
//! ```toml
//! ninelives-nats = { version = "0.1", features = ["client"] }
//! ```
//!
//! ```rust
//! use ninelives_nats::NatsSink;
//! # use ninelives::telemetry::PolicyEvent;
//! # async fn demo() -> Result<(), Box<dyn std::error::Error>> {
//! let sink = NatsSink::new("nats://127.0.0.1:4222", "policy.events")?;
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
    client: nats::asynk::Connection,
}

impl NatsSink {
    /// Create a sink using an existing NATS async connection.
    pub fn new(client: nats::asynk::Connection, subject: impl Into<String>) -> Self {
        Self { subject: subject.into(), client }
    }
}

impl tower_service::Service<PolicyEvent> for NatsSink {
    type Response = ();
    type Error = Infallible;
    type Future = Pin<Box<dyn std::future::Future<Output = Result<(), Self::Error>> + Send>>;

    #[cfg_attr(not(feature = "client"), allow(unused_variables))]
    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    #[cfg_attr(not(feature = "client"), allow(unused_variables))]
    fn call(&mut self, event: PolicyEvent) -> Self::Future {
        #[cfg(feature = "client")]
        let fut = {
            let subject = self.subject.clone();
            let mut client = self.client.clone();
            let payload =
                serde_json::to_vec(&event_to_json(&event)).unwrap_or_else(|_| b"{}".to_vec());
            Box::pin(async move {
                let _ = client.publish(subject, payload).await;
                Ok(())
            })
        };

        #[cfg(not(feature = "client"))]
        let fut = {
            // Still touch the event to ensure serialization paths stay valid even when the client
            // feature is off (keeps compilation honest for downstream users).
            let _ = serde_json::to_vec(&event_to_json(&event));
            Box::pin(async move { Ok(()) })
        };

        fut
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
