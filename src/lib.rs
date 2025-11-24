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
//! use ninelives::{RetryPolicy, Backoff, Jitter, ResilienceError};
//! use std::time::Duration;
//!
//! #[tokio::main]
//! async fn main() {
//!     let policy = RetryPolicy::builder()
//!         .max_attempts(3)
//!         .backoff(Backoff::exponential(Duration::from_secs(1)))
//!         .with_jitter(Jitter::full())
//!         .build();
//!
//!     let result = policy.execute(|| async {
//!         // Your async operation here
//!         Ok::<_, ResilienceError<std::io::Error>>(())
//!     }).await;
//! }
//! ```

pub mod error;
pub mod sleeper;
pub mod backoff;
pub mod jitter;
pub mod retry;
pub mod circuit_breaker;
pub mod bulkhead;
pub mod timeout;
pub mod stack;

// Re-exports
pub use error::ResilienceError;
pub use sleeper::{Sleeper, TokioSleeper, InstantSleeper, TrackingSleeper};
pub use backoff::Backoff;
pub use jitter::Jitter;
pub use retry::{RetryPolicy, RetryPolicyBuilder};
pub use circuit_breaker::{CircuitBreakerPolicy, CircuitBreakerConfig, CircuitState};
pub use bulkhead::BulkheadPolicy;
pub use timeout::TimeoutPolicy;
pub use stack::{ResilienceStack, ResilienceStackBuilder};
