#![forbid(unsafe_code)]
#![deny(warnings)]
#![cfg_attr(not(test), deny(clippy::all))]

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
//! - **Policy composition** via builder pattern
//! - **Lock-free implementations** using atomics
//!
//! ## Quick Start
//!
//! ```rust
//! use std::sync::atomic::{AtomicUsize, Ordering};
//! use std::sync::Arc;
//! use std::time::Duration;
//! use ninelives::{Backoff, Jitter, ResilienceError, RetryPolicy};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), ResilienceError<std::io::Error>> {
//!     let attempts = Arc::new(AtomicUsize::new(0));
//!
//!     let policy = RetryPolicy::builder()
//!         .max_attempts(3).expect("max_attempts > 0")
//!         .backoff(Backoff::exponential(Duration::from_millis(200)))
//!         .with_jitter(Jitter::full())
//!         .build();
//!
//!     policy
//!         .execute(|| {
//!             let attempts = attempts.clone();
//!             async move {
//!                 let n = attempts.fetch_add(1, Ordering::SeqCst);
//!                 if n < 2 {
//!                     Err(ResilienceError::Inner(std::io::Error::new(
//!                         std::io::ErrorKind::Other,
//!                         "transient failure",
//!                     )))
//!                 } else {
//!                     Ok::<_, ResilienceError<std::io::Error>>(())
//!                 }
//!             }
//!         })
//!         .await?;
//!
//!     Ok(())
//! }
//! ```

pub mod backoff;
pub mod bulkhead;
pub mod circuit_breaker;
pub mod clock;
pub mod error;
pub mod jitter;
pub mod retry;
pub mod sleeper;
pub mod stack;
pub mod timeout;

// Re-exports
pub use backoff::Backoff;
pub use bulkhead::BulkheadPolicy;
pub use circuit_breaker::{CircuitBreakerConfig, CircuitBreakerPolicy, CircuitState};
pub use clock::{Clock, MonotonicClock};
pub use error::ResilienceError;
pub use jitter::Jitter;
pub use retry::{RetryPolicy, RetryPolicyBuilder};
/// Sleep abstractions: `Sleeper` trait, `TokioSleeper` for production, `InstantSleeper`/`TrackingSleeper` for tests.
pub use sleeper::{InstantSleeper, Sleeper, TokioSleeper, TrackingSleeper};
pub use stack::{ResilienceStack, ResilienceStackBuilder};
pub use timeout::TimeoutPolicy;
