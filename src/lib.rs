#![forbid(unsafe_code)]
#![allow(missing_docs)]

//! # Nine Lives 🐱
//!
//! Resilience patterns for Rust: retry policies, circuit breakers,
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
//! ### Basic Usage - Single Layer
//!
//! ```rust
//! use ninelives::prelude::*;
//! use std::time::Duration;
//! use tower::{Service, ServiceBuilder};
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a simple service
//! let mut svc = ServiceBuilder::new()
//!     .layer(TimeoutLayer::new(Duration::from_secs(1))?)
//!     .service_fn(|req: &'static str| async move {
//!         Ok::<_, std::io::Error>(req)
//!     });
//!
//! let response = svc.call("hello").await?;
//! assert_eq!(response, "hello");
//! # Ok(())
//! # }
//! ```
//!
//! ### Algebraic Composition - The Power of `Policy`
//!
//! Compose layers using intuitive operators:
//!
//! ```rust
//! use ninelives::prelude::*;
//! use std::time::Duration;
//! use tower::{Service, ServiceBuilder};
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Define resilience strategy using algebra
//! // Try fast timeout, fallback to longer timeout
//! let fast = Policy(TimeoutLayer::new(Duration::from_millis(100))?);
//! let slow = Policy(TimeoutLayer::new(Duration::from_secs(5))?);
//! let policy = fast | slow;  // Fallback operator
//!
//! // Apply to any service
//! let mut svc = ServiceBuilder::new()
//!     .layer(policy)
//!     .service_fn(|req: &'static str| async move {
//!         Ok::<_, std::io::Error>(req)
//!     });
//!
//! let response = svc.call("hello").await?;
//! # Ok(())
//! # }
//! ```
//!
//! ### Complex Composition
//!
//! Combine multiple strategies with precedence:
//!
//! ```rust
//! use ninelives::prelude::*;
//! use std::time::Duration;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Aggressive: fast timeout only
//! let aggressive = Policy(TimeoutLayer::new(Duration::from_millis(50))?);
//!
//! // Defensive: nested timeouts for resilience
//! let defensive = Policy(TimeoutLayer::new(Duration::from_secs(10))?)
//!               + Policy(TimeoutLayer::new(Duration::from_secs(5))?);
//!
//! // Try aggressive first, fall back to defensive
//! // Operator precedence: + binds tighter than |
//! let _policy = aggressive | defensive;
//! // This creates: Policy(Timeout50ms) | (Policy(Timeout10s) + Policy(Timeout5s))
//! # Ok(())
//! # }
//! ```
//!
//! ## Algebraic Operators
//!
//! - **`Policy(A) + Policy(B)`** - Sequential composition: `A` wraps `B`
//! - **`Policy(A) | Policy(B)`** - Fallback: try `A`, fall back to `B` on error
//! - **`Policy(A) & Policy(B)`** - Fork-join: try both concurrently, return first success
//!
//! **Precedence:** `&` > `+` > `|` (like `*` > `+` > bitwise-or in math)
//!
//! Example: `A | B + C & D` is parsed as `A | (B + (C & D))`
//!
//! ## Available Layers
//!
//! - **[`TimeoutLayer`]** - Enforce time limits on operations
//! - **[`RetryLayer`]** - Retry failed operations with backoff
//! - **[`CircuitBreakerLayer`]** - Prevent cascading failures
//! - **[`BulkheadLayer`]** - Limit concurrent requests
//!
//! For more examples, see the algebra module documentation.

pub mod adaptive;
mod algebra;
mod backoff;
mod bulkhead;
mod circuit_breaker;
pub mod circuit_breaker_registry;
mod clock;
pub mod control;
mod error;
mod jitter;
mod retry;
mod sleeper;
// stack module removed in favor of tower-native algebra
pub mod telemetry;
mod timeout;

// Re-exports
pub use algebra::{
    CombinedLayer, FallbackLayer, FallbackService, ForkJoinLayer, ForkJoinService, Policy,
};
pub use backoff::{
    Backoff, BackoffError, BackoffStrategy, ConstantBackoff, ExponentialBackoff, LinearBackoff,
    MAX_BACKOFF,
};
pub use bulkhead::BulkheadLayer;
pub use bulkhead::{BulkheadError, BulkheadPolicy};
pub use circuit_breaker::{
    CircuitBreakerConfig, CircuitBreakerError, CircuitBreakerLayer, CircuitState,
};
pub use clock::{Clock, MonotonicClock};
pub use control::transport::{Transport, TransportEnvelope, TransportRouter};
pub use control::AuthorizationLayer;
pub use error::ResilienceError;
pub use jitter::Jitter;
pub use retry::{BuildError, RetryLayer, RetryPolicy, RetryPolicyBuilder, RetryService};
pub use sleeper::{InstantSleeper, Sleeper, TokioSleeper, TrackingSleeper};
pub use timeout::{TimeoutError, TimeoutLayer, TimeoutPolicy, MAX_TIMEOUT};

pub mod prelude;
