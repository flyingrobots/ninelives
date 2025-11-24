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
//! async fn flaky_operation(
//!     attempts: Arc<AtomicUsize>,
//! ) -> Result<(), ResilienceError<std::io::Error>> {
//!     let n = attempts.fetch_add(1, Ordering::SeqCst);
//!     if n < 2 {
//!         Err(ResilienceError::Inner(std::io::Error::new(
//!             std::io::ErrorKind::Other,
//!             "transient failure",
//!         )))
//!     } else {
//!         Ok(())
//!     }
//! }
//!
//! #[tokio::main]
//! async fn main() -> Result<(), ResilienceError<std::io::Error>> {
//!     let attempts = Arc::new(AtomicUsize::new(0));
//!
//!     let policy = RetryPolicy::builder()
//!         .max_attempts(3)
//!         .backoff(Backoff::exponential(Duration::from_millis(200)))
//!         .with_jitter(Jitter::full())
//!         .build()
//!         .expect("valid retry policy");
//!
//!     policy.execute(|| flaky_operation(attempts.clone())).await?;
//!     Ok(())
//! }
//! ```

mod backoff;
mod bulkhead;
mod circuit_breaker;
mod clock;
mod error;
mod jitter;
mod retry;
mod sleeper;
mod stack;
mod timeout;

// Re-exports
pub use backoff::Backoff;
pub use bulkhead::{BulkheadError, BulkheadPolicy};
pub use circuit_breaker::{
    CircuitBreakerConfig, CircuitBreakerError, CircuitBreakerPolicy, CircuitState,
};
pub use clock::{Clock, MonotonicClock};
pub use error::ResilienceError;
pub use jitter::Jitter;
pub use retry::{BuildError as RetryBuildError, RetryPolicy, RetryPolicyBuilder};
pub use sleeper::{InstantSleeper, Sleeper, TokioSleeper, TrackingSleeper};
pub use stack::{ResilienceStack, ResilienceStackBuilder, StackError};
pub use timeout::{TimeoutError, TimeoutPolicy, MAX_TIMEOUT};
