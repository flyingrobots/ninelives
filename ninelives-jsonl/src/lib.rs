//! JSONL sink for `ninelives`. Writes one event per line.
//! Always writes; bring your own path.

use ninelives::telemetry::{PolicyEvent, TelemetrySink};
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
