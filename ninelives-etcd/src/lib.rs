//! etcd telemetry sink for `ninelives` (companion crate).
//! Bring your own `etcd_client::Client`; events are stored as JSON under a prefix.

use ninelives::telemetry::{PolicyEvent, TelemetrySink};
use serde_json::json;
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
        let key = format!(
            "{}/{}-{}",
            self.prefix,
            chrono::Utc::now().timestamp_nanos_opt().expect("valid timestamp"),
            uuid::Uuid::new_v4()
        );
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

fn event_to_json(event: &PolicyEvent) -> serde_json::Value {
    use ninelives::telemetry::{
        BulkheadEvent, CircuitBreakerEvent, RequestOutcome, RetryEvent, TimeoutEvent,
    };
    match event {
        PolicyEvent::Retry(r) => match r {
            RetryEvent::Attempt { attempt, delay } => {
                json!({ "kind": "retry_attempt", "attempt": attempt, "delay_ms": delay.as_millis() as u64 })
            }
            RetryEvent::Exhausted { total_attempts, total_duration } => {
                json!({ "kind": "retry_exhausted", "attempts": total_attempts, "duration_ms": total_duration.as_millis() as u64 })
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
                json!({ "kind": "timeout", "timeout_ms": timeout.as_millis() as u64 })
            }
        },
        PolicyEvent::Request(r) => match r {
            RequestOutcome::Success { duration } => {
                json!({ "kind": "request_success", "duration_ms": duration.as_millis() as u64 })
            }
            RequestOutcome::Failure { duration } => {
                json!({ "kind": "request_failure", "duration_ms": duration.as_millis() as u64 })
            }
        },
    }
}
