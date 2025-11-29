//! OTLP telemetry sink for `ninelives`.
//! Bring your own `opentelemetry_sdk::logs::LoggerProvider`; events are emitted as OTLP logs.

use ninelives::telemetry::{PolicyEvent, TelemetrySink};
use opentelemetry::logs::{AnyValue, LogRecord, Logger, LoggerProvider, Severity};
use std::convert::Infallible;
use std::pin::Pin;
use std::task::{Context, Poll};

/// OTLP sink that emits PolicyEvents as structured logs.
///
/// Example usage:
/// ```ignore
/// use opentelemetry_sdk::logs::LoggerProvider;
/// use ninelives_otlp::OtlpSink;
///
/// let provider = LoggerProvider::builder().build();
/// let sink = OtlpSink::new(provider);
/// ```
#[derive(Clone, Debug)]
pub struct OtlpSink<P> {
    provider: P,
}

impl<P> OtlpSink<P>
where
    P: LoggerProvider + Clone + Send + Sync + 'static,
{
    /// Create a sink from an existing OTLP logger provider
    pub fn new(provider: P) -> Self {
        Self { provider }
    }
}

impl<P> tower_service::Service<PolicyEvent> for OtlpSink<P>
where
    P: LoggerProvider + Clone + Send + Sync + 'static,
    P::Logger: Send,
{
    type Response = ();
    type Error = Infallible;
    type Future = Pin<Box<dyn std::future::Future<Output = Result<(), Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, event: PolicyEvent) -> Self::Future {
        let logger = self.provider.logger("ninelives");
        Box::pin(async move {
            let (severity, body, event_kind, numeric_attrs) = map_event(&event);
            let mut record = logger.create_log_record();
            record.set_severity_number(severity);
            record.set_body(AnyValue::from(body));
            record.add_attribute("component", "ninelives");
            record.add_attribute("event_kind", event_kind);
            record.add_attributes(numeric_attrs);
            logger.emit(record);
            Ok(())
        })
    }
}

impl<P> TelemetrySink for OtlpSink<P>
where
    P: LoggerProvider + Clone + Send + Sync + 'static,
    P::Logger: Send,
{
    type SinkError = Infallible;
}

fn map_event(event: &PolicyEvent) -> (Severity, String, &'static str, Vec<(&'static str, i64)>) {
    use ninelives::telemetry::{
        BulkheadEvent, CircuitBreakerEvent, RequestOutcome, RetryEvent, TimeoutEvent,
    };

    let event_kind = kind(event);

    match event {
        PolicyEvent::Retry(RetryEvent::Attempt { attempt, delay }) => {
            let attrs = vec![("attempt", *attempt as i64), ("delay_ms", delay.as_millis() as i64)];
            (Severity::Info, "retry_attempt".to_string(), event_kind, attrs)
        }
        PolicyEvent::Retry(RetryEvent::Exhausted { total_attempts, total_duration }) => {
            let attrs = vec![
                ("total_attempts", *total_attempts as i64),
                ("total_duration_ms", total_duration.as_millis() as i64),
            ];
            (Severity::Warn, "retry_exhausted".to_string(), event_kind, attrs)
        }
        PolicyEvent::CircuitBreaker(CircuitBreakerEvent::Opened { failure_count }) => {
            let attrs = vec![("failure_count", *failure_count as i64)];
            (Severity::Warn, "circuit_opened".to_string(), event_kind, attrs)
        }
        PolicyEvent::CircuitBreaker(CircuitBreakerEvent::HalfOpen) => {
            (Severity::Info, "circuit_half_open".to_string(), event_kind, vec![])
        }
        PolicyEvent::CircuitBreaker(CircuitBreakerEvent::Closed) => {
            (Severity::Info, "circuit_closed".to_string(), event_kind, vec![])
        }
        PolicyEvent::Bulkhead(BulkheadEvent::Acquired { active_count, max_concurrency }) => {
            let attrs = vec![("active", *active_count as i64), ("max", *max_concurrency as i64)];
            (Severity::Info, "bulkhead_acquired".to_string(), event_kind, attrs)
        }
        PolicyEvent::Bulkhead(BulkheadEvent::Rejected { active_count, max_concurrency }) => {
            let attrs = vec![("active", *active_count as i64), ("max", *max_concurrency as i64)];
            (Severity::Warn, "bulkhead_rejected".to_string(), event_kind, attrs)
        }
        PolicyEvent::Timeout(TimeoutEvent::Occurred { timeout }) => {
            let attrs = vec![("timeout_ms", timeout.as_millis() as i64)];
            (Severity::Warn, "timeout".to_string(), event_kind, attrs)
        }
        PolicyEvent::Request(RequestOutcome::Success { duration }) => {
            let attrs = vec![("duration_ms", duration.as_millis() as i64)];
            (Severity::Info, "request_success".to_string(), event_kind, attrs)
        }
        PolicyEvent::Request(RequestOutcome::Failure { duration }) => {
            let attrs = vec![("duration_ms", duration.as_millis() as i64)];
            (Severity::Warn, "request_failure".to_string(), event_kind, attrs)
        }
    }
}

fn kind(event: &PolicyEvent) -> &'static str {
    match event {
        PolicyEvent::Retry(_) => "retry",
        PolicyEvent::CircuitBreaker(_) => "circuit_breaker",
        PolicyEvent::Bulkhead(_) => "bulkhead",
        PolicyEvent::Timeout(_) => "timeout",
        PolicyEvent::Request(_) => "request",
    }
}
