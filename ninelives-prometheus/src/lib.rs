//! Prometheus metrics sink for `ninelives`.
//! Collects counters in-process; expose via your HTTP endpoint using `prometheus::TextEncoder`.
//! Default build is no-op; enable `client` to record metrics.

use ninelives::telemetry::{PolicyEvent, TelemetrySink};
use std::convert::Infallible;
use std::pin::Pin;
use std::task::{Context, Poll};

#[derive(Clone, Debug)]
pub struct PrometheusSink {
    #[cfg(feature = "client")]
    registry: prometheus::Registry,
    #[cfg(feature = "client")]
    counter: prometheus::IntCounterVec,
}

impl PrometheusSink {
    pub fn new() -> Self {
        #[cfg(feature = "client")]
        {
            let registry = prometheus::Registry::new();
            let counter = prometheus::IntCounterVec::new(
                prometheus::Opts::new("ninelives_events_total", "Policy events"),
                &[
                    "policy",
                    "event",
                ],
            )
            .expect("create counter");
            registry.register(Box::new(counter.clone())).ok();
            return Self { registry, counter };
        }
        #[cfg(not(feature = "client"))]
        {
            Self {}
        }
    }

    /// Expose the registry for HTTP scraping.
    #[cfg(feature = "client")]
    pub fn registry(&self) -> &prometheus::Registry {
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
        #[cfg(feature = "client")]
        {
            let labels = match &event {
                PolicyEvent::Retry(_) => ("retry", "event"),
                PolicyEvent::CircuitBreaker(_) => ("circuit", "event"),
                PolicyEvent::Bulkhead(_) => ("bulkhead", "event"),
                PolicyEvent::Timeout(_) => ("timeout", "event"),
                PolicyEvent::Request(_) => ("request", "event"),
            };
            let c = self.counter.clone();
            let (p, e) = labels;
            return Box::pin(async move {
                c.with_label_values(&[p, e]).inc();
                Ok(())
            });
        }
        #[cfg(not(feature = "client"))]
        {
            return Box::pin(async move { Ok(()) });
        }
    }
}

impl TelemetrySink for PrometheusSink {
    type SinkError = Infallible;
}
