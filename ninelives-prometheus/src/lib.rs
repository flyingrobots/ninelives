//! Prometheus metrics sink for `ninelives`.
//! Bring your own `prometheus::Registry`; counters are registered and incremented.

use ninelives::prelude::{
    BulkheadEvent, CircuitBreakerEvent, RequestOutcome, RetryEvent, TimeoutEvent,
};
use ninelives::telemetry::{
    BulkheadEvent, CircuitBreakerEvent, PolicyEvent, RequestOutcome, RetryEvent, TelemetrySink,
    TimeoutEvent,
};
use prometheus::{IntCounterVec, Registry};
use std::convert::Infallible;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::future::{ready, Ready};

#[derive(Clone, Debug)]
pub struct PrometheusSink {
    registry: Arc<Registry>,
    counter: IntCounterVec,
}

impl PrometheusSink {
    /// Create a sink and register counters into the provided registry.
    ///
    /// # Errors
    /// Returns an error if the metric cannot be registered (e.g. name conflict).
    pub fn new<R: Into<Arc<Registry>>>(registry: R) -> Result<Self, prometheus::Error> {
        let registry = registry.into();
        let counter = IntCounterVec::new(
            prometheus::Opts::new("ninelives_events_total", "Policy events"),
            &["policy", "event"],
        )?;
        registry.register(Box::new(counter.clone()))?;
        Ok(Self { registry, counter })
    }

    /// Expose the registry for HTTP scraping.
    pub fn registry(&self) -> &Registry {
        &self.registry
    }
}

impl tower_service::Service<PolicyEvent> for PrometheusSink {
    type Response = ();
    type Error = Infallible;
    type Future = Ready<Result<(), Self::Error>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, event: PolicyEvent) -> Self::Future {
        let (policy_label, event_label) = match &event {
            PolicyEvent::Retry(r) => (
                "retry",
                match r {
                    RetryEvent::Attempt { .. } => "attempt",
                    RetryEvent::Exhausted { .. } => "exhausted",
                },
            ),
            PolicyEvent::CircuitBreaker(c) => (
                "circuit_breaker",
                match c {
                    CircuitBreakerEvent::Opened { .. } => "opened",
                    CircuitBreakerEvent::HalfOpen => "half_open",
                    CircuitBreakerEvent::Closed => "closed",
                },
            ),
            PolicyEvent::Bulkhead(b) => (
                "bulkhead",
                match b {
                    BulkheadEvent::Acquired { .. } => "acquired",
                    BulkheadEvent::Rejected { .. } => "rejected",
                },
            ),
            PolicyEvent::Timeout(t) => (
                "timeout",
                match t {
                    TimeoutEvent::Occurred { .. } => "occurred",
                },
            ),
            PolicyEvent::Request(r) => (
                "request",
                match r {
                    RequestOutcome::Success { .. } => "success",
                    RequestOutcome::Failure { .. } => "failure",
                },
            ),
        };
        let c = self.counter.clone();
        c.with_label_values(&[policy_label, event_label]).inc();
        ready(Ok(()))
    }
}

impl TelemetrySink for PrometheusSink {
    type SinkError = Infallible;
}
