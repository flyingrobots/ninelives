//! Prometheus metrics sink for `ninelives`.
//! Bring your own `prometheus::Registry`; counters are registered and incremented.

use ninelives::telemetry::{PolicyEvent, TelemetrySink};
use prometheus::{IntCounterVec, Registry};
use std::convert::Infallible;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

#[derive(Clone, Debug)]
pub struct PrometheusSink {
    registry: Arc<Registry>,
    counter: IntCounterVec,
}

impl PrometheusSink {
    /// Create a sink and register counters into the provided registry.
    pub fn new(registry: Registry) -> Self {
        let counter = IntCounterVec::new(
            prometheus::Opts::new("ninelives_events_total", "Policy events"),
            &["policy", "event"],
        )
        .expect("create counter");
        let registry = Arc::new(registry);
        registry.register(Box::new(counter.clone())).ok();
        Self { registry, counter }
    }

    /// Expose the registry for HTTP scraping.
    pub fn registry(&self) -> &Registry {
        &self.registry
    }
}

impl tower_service::Service<PolicyEvent> for PrometheusSink {
    type Response = ();
    type Error = Infallible;
    type Future = Pin<Box<dyn std::future::Future<Output = Result<(), Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, event: PolicyEvent) -> Self::Future {
        let labels = match &event {
            PolicyEvent::Retry(_) => ("retry", "event"),
            PolicyEvent::CircuitBreaker(_) => ("circuit", "event"),
            PolicyEvent::Bulkhead(_) => ("bulkhead", "event"),
            PolicyEvent::Timeout(_) => ("timeout", "event"),
            PolicyEvent::Request(_) => ("request", "event"),
        };
        let c = self.counter.clone();
        let (p, e) = labels;
        Box::pin(async move {
            c.with_label_values(&[p, e]).inc();
            Ok(())
        })
    }
}

impl TelemetrySink for PrometheusSink {
    type SinkError = Infallible;
}
