#![forbid(unsafe_code)]

//! # Nine Lives 🐱
//!
//! Production-grade resilience patterns for Rust: retry policies, circuit breakers,
//! bulkheads, and timeouts.
//!
//! ## Features
//!
//! - **Retry policies** with backoff strategies (constant, linear, exponential)
//! - **Circuit breakers** with half-open state recovery
//! - **Bulkheads** for concurrency limiting and resource isolation
//! - **Timeout policies** integrated with tokio
//! - **Policy composition** via tower-native layers and algebraic operators
//! - **Lock-free implementations** using atomics
//!
//! ## Quick Start
//!
//! ```rust
//! use ninelives::prelude::*;
//! use std::time::Duration;
//! use tower_layer::Layer;
//! use tower_service::Service;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), ResilienceError<std::io::Error>> {
//!     let timeout = TimeoutLayer::new(Duration::from_secs(1)).expect("timeout layer");
//!     let mut svc = timeout.layer(tower::service_fn(|req: &'static str| async move {
//!         Ok::<_, std::io::Error>(req)
//!     }));
//!     let _ = svc.call("hello").await.unwrap();
//!     Ok(())
//! }
//! ```

mod algebra;
mod backoff;
mod bulkhead;
mod circuit_breaker;
mod clock;
mod error;
mod jitter;
mod retry;
mod sleeper;
// stack module removed in favor of tower-native algebra
mod timeout;

// Re-exports
pub use algebra::{CombinedLayer, FallbackLayer, Policy};
pub use backoff::{
    Backoff, BackoffError, BackoffStrategy, ConstantBackoff, ExponentialBackoff, LinearBackoff,
    MAX_BACKOFF,
};
pub use bulkhead::{BulkheadError, BulkheadPolicy};
pub use circuit_breaker::{
    CircuitBreakerConfig, CircuitBreakerError, CircuitBreakerLayer, CircuitState,
};
pub use clock::{Clock, MonotonicClock};
pub use error::ResilienceError;
pub use jitter::Jitter;
pub use retry::{BuildError, RetryPolicy, RetryPolicyBuilder};
pub use sleeper::{InstantSleeper, Sleeper, TokioSleeper, TrackingSleeper};
pub use timeout::{TimeoutError, TimeoutPolicy, MAX_TIMEOUT};

pub mod prelude;
