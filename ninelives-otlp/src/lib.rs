//! OTLP telemetry sink for `ninelives`.
//! Default build is no-op; enable `client` to export events as OTLP logs.

use ninelives::telemetry::{PolicyEvent, TelemetrySink};
use std::convert::Infallible;
use std::pin::Pin;
use std::task::{Context, Poll};

#[cfg(feature = "client")]
use opentelemetry::{global, KeyValue};
#[cfg(feature = "client")]
use opentelemetry::logs::{AnyValue, LogEmitterProvider, Logger, Severity};
#[cfg(feature = "client")]
use opentelemetry_otlp::WithExportConfig;

#[derive(Clone, Debug)]
pub struct OtlpSink {
    #[cfg(feature = "client")]
    logger: Logger,
}

impl OtlpSink {
    pub fn new() -> Self {
        #[cfg(feature = "client")]
        {
            // Build a simple OTLP logger pipeline
            let provider = opentelemetry_otlp::new_pipeline()
                .logging()
                .with_export_config(opentelemetry_otlp::ExportConfig::default())
                .install_simple()
                .expect("install otlp logger");
            let logger = provider.logger("ninelives-otlp");
            return Self { logger };
        }
        #[cfg(not(feature = "client"))]
        {
            Self {}
        }
    }
}

impl tower_service::Service<PolicyEvent> for OtlpSink {
    type Response = ();
    type Error = Infallible;
    type Future = Pin<Box<dyn std::future::Future<Output = Result<(), Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, event: PolicyEvent) -> Self::Future {
        #[cfg(feature = "client")]
        {
            let logger = self.logger.clone();
            return Box::pin(async move {
                let (severity, attrs, body) = map_event(&event);
                logger.emit(
                    opentelemetry::logs::LogRecord::builder()
                        .with_severity(severity)
                        .with_body(AnyValue::from(body))
                        .with_attributes(attrs)
                        .build(),
                );
                Ok(())
            });
        }
        #[cfg(not(feature = "client"))]
        {
            return Box::pin(async move { Ok(()) });
        }
    }
}

impl TelemetrySink for OtlpSink {
    type SinkError = Infallible;
}

#[cfg(feature = "client")]
fn map_event(event: &PolicyEvent) -> (Severity, Vec<KeyValue>, String) {
    use ninelives::telemetry::{BulkheadEvent, CircuitBreakerEvent, RequestOutcome, RetryEvent, TimeoutEvent};

    let mut attrs = vec![KeyValue::new("component", "ninelives"), KeyValue::new("event_kind", kind(event))];

    match event {
        PolicyEvent::Retry(RetryEvent::Attempt { attempt, delay }) => {
            attrs.push(KeyValue::new("attempt", (*attempt as i64).into()));
            attrs.push(KeyValue::new("delay_ms", delay.as_millis() as i64));
            (Severity::Info, attrs, "retry_attempt".to_string())
        }
        PolicyEvent::Retry(RetryEvent::Exhausted { total_attempts, total_duration }) => {
            attrs.push(KeyValue::new("total_attempts", (*total_attempts as i64).into()));
            attrs.push(KeyValue::new("total_duration_ms", total_duration.as_millis() as i64));
            (Severity::Warn, attrs, "retry_exhausted".to_string())
        }
        PolicyEvent::CircuitBreaker(CircuitBreakerEvent::Opened { failure_count }) => {
            attrs.push(KeyValue::new("failure_count", (*failure_count as i64).into()));
            (Severity::Warn, attrs, "circuit_opened".to_string())
        }
        PolicyEvent::CircuitBreaker(CircuitBreakerEvent::HalfOpen) => (Severity::Info, attrs, "circuit_half_open".to_string()),
        PolicyEvent::CircuitBreaker(CircuitBreakerEvent::Closed) => (Severity::Info, attrs, "circuit_closed".to_string()),
        PolicyEvent::Bulkhead(BulkheadEvent::Acquired { active_count, max_concurrency }) => {
            attrs.push(KeyValue::new("active", (*active_count as i64).into()));
            attrs.push(KeyValue::new("max", (*max_concurrency as i64).into()));
            (Severity::Info, attrs, "bulkhead_acquired".to_string())
        }
        PolicyEvent::Bulkhead(BulkheadEvent::Rejected { active_count, max_concurrency }) => {
            attrs.push(KeyValue::new("active", (*active_count as i64).into()));
            attrs.push(KeyValue::new("max", (*max_concurrency as i64).into()));
            (Severity::Warn, attrs, "bulkhead_rejected".to_string())
        }
        PolicyEvent::Timeout(TimeoutEvent::Occurred { timeout }) => {
            attrs.push(KeyValue::new("timeout_ms", timeout.as_millis() as i64));
            (Severity::Warn, attrs, "timeout".to_string())
        }
        PolicyEvent::Request(RequestOutcome::Success { duration }) => {
            attrs.push(KeyValue::new("duration_ms", duration.as_millis() as i64));
            (Severity::Info, attrs, "request_success".to_string())
        }
        PolicyEvent::Request(RequestOutcome::Failure { duration }) => {
            attrs.push(KeyValue::new("duration_ms", duration.as_millis() as i64));
            (Severity::Warn, attrs, "request_failure".to_string())
        }
    }
}

#[cfg(feature = "client")]
fn kind(event: &PolicyEvent) -> &'static str {
    match event {
        PolicyEvent::Retry(_) => "retry",
        PolicyEvent::CircuitBreaker(_) => "circuit_breaker",
        PolicyEvent::Bulkhead(_) => "bulkhead",
        PolicyEvent::Timeout(_) => "timeout",
        PolicyEvent::Request(_) => "request",
    }
}
