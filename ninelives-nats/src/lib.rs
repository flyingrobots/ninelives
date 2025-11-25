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
use std::convert::Infallible;
use std::pin::Pin;
use std::task::{Context, Poll};

#[derive(Clone, Debug)]
pub struct NatsSink {
    subject: String,
    #[cfg(feature = "client")]
    client: nats::asynk::Connection,
}

impl NatsSink {
    pub fn new<S: Into<String>>(server: S, subject: S) -> Result<Self, Box<dyn std::error::Error>> {
        let subject = subject.into();
        #[cfg(feature = "client")]
        {
            let client = nats::asynk::connect(server.into())?;
            return Ok(Self { subject, client });
        }
        #[cfg(not(feature = "client"))]
        {
            let _ = server; // unused
            Ok(Self { subject, ..Self::noop() })
        }
    }

    #[cfg(not(feature = "client"))]
    fn noop() -> Self {
        Self { subject: String::new() }
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
        #[cfg(feature = "client")]
        let fut = {
            let subject = self.subject.clone();
            let mut client = self.client.clone();
            let payload = format!("{:?}", event).into_bytes();
            Box::pin(async move {
                let _ = client.publish(subject, payload).await;
                Ok(())
            })
        };

        #[cfg(not(feature = "client"))]
        let fut = Box::pin(async move { Ok(()) });

        fut
    }
}

impl TelemetrySink for NatsSink {
    type SinkError = Infallible;
}
