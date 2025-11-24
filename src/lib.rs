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
//! use ninelives::{
//!     Backoff, BulkheadPolicy, CircuitBreakerPolicy, Jitter, ResilienceError, ResilienceStack,
//!     ResilienceStackBuilder, RetryPolicy, TimeoutPolicy,
//! };
//!
//! async fn flaky_operation(
//!     attempts: Arc<AtomicUsize>,
//! ) -> Result<(), ResilienceError<std::io::Error>> {
//!     let n = attempts.fetch_add(1, Ordering::Relaxed);
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
//!     // Configure individual policies.
//!     let retry = RetryPolicy::builder()
//!         .max_attempts(3)
//!         .backoff(Backoff::exponential(Duration::from_millis(200)))
//!         .with_jitter(Jitter::full())
//!         .build()
//!         .expect("valid retry policy");
//!     let timeout = TimeoutPolicy::new(Duration::from_secs(2)).expect("valid timeout");
//!     let bulkhead = BulkheadPolicy::new(32).expect("valid bulkhead");
//!     let circuit_breaker =
//!         CircuitBreakerPolicy::new(5, Duration::from_secs(30)).expect("valid breaker");
//!
//!     // Compose via the stack builder (Retry → CircuitBreaker → Bulkhead → Timeout).
//!     let stack: ResilienceStack<std::io::Error> = ResilienceStackBuilder::new()
//!         .retry(retry)
//!         .circuit_breaker(5, Duration::from_secs(30)).expect("valid breaker config")
//!         .bulkhead(32).expect("valid bulkhead config")
//!         .timeout(Duration::from_secs(2)).expect("valid timeout config")
//!         .build()
//!         .expect("valid stack");
//!
//!     stack.execute(|| flaky_operation(attempts.clone())).await?;
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
pub use backoff::{
    Backoff, BackoffError, BackoffStrategy, ConstantBackoff, ExponentialBackoff, LinearBackoff,
    MAX_BACKOFF,
};
pub use bulkhead::{BulkheadError, BulkheadPolicy};
pub use circuit_breaker::{
    CircuitBreakerConfig, CircuitBreakerError, CircuitBreakerPolicy, CircuitState,
};
pub use clock::{Clock, MonotonicClock};
pub use error::ResilienceError;
pub use jitter::Jitter;
pub use retry::{BuildError, RetryPolicy, RetryPolicyBuilder};
pub use sleeper::{InstantSleeper, Sleeper, TokioSleeper, TrackingSleeper};
pub use stack::{ResilienceStack, ResilienceStackBuilder, StackError};
pub use timeout::{TimeoutError, TimeoutPolicy, MAX_TIMEOUT};

pub mod prelude;
