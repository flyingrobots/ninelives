//! Resilience stack builder for composing policies
//!
//! Order of application (outer → inner → operation):
//! `Retry → CircuitBreaker → Bulkhead → Timeout → Operation`.
//!
//! Typical roles:
//! - **Retry**: re-run retryable failures with backoff/jitter.
//! - **CircuitBreaker**: open on repeated failures and probe recovery via half-open state.
//! - **Bulkhead**: cap concurrent work to isolate overload.
//! - **Timeout**: bound execution time of the inner operation.
//!
//! Example (no_run):
//! ```no_run
//! use std::time::Duration;
//! use ninelives as your_crate;
//! use your_crate::{Backoff, Jitter, ResilienceError, ResilienceStack, RetryPolicy};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), ResilienceError<std::io::Error>> {
//!     let retry = RetryPolicy::builder()
//!         .max_attempts(3)
//!         .backoff(Backoff::exponential(Duration::from_millis(100)))
//!         .with_jitter(Jitter::full())
//!         .build();
//!
//!     let stack = ResilienceStack::<std::io::Error>::new()
//!         .timeout(Duration::from_secs(2))
//!         .bulkhead(32)
//!         .circuit_breaker(5, Duration::from_secs(30))
//!         .retry(retry)
//!         .build();
//!
//!     stack
//!         .execute(|| async {
//!             // your async operation
//!             Ok::<_, ResilienceError<std::io::Error>>(())
//!         })
//!         .await?;
//!     Ok(())
//! }
//! ```

use crate::{
    BulkheadPolicy, CircuitBreakerConfig, CircuitBreakerPolicy, ResilienceError, RetryPolicy,
    TimeoutPolicy,
};
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

/// Default timeout applied when none is specified (seconds).
pub const DEFAULT_TIMEOUT_SECS: u64 = 30;
/// Default bulkhead concurrency limit when unspecified.
pub const DEFAULT_BULKHEAD_MAX_CONCURRENT: usize = 100;
/// Default circuit-breaker failure threshold before opening.
pub const DEFAULT_CIRCUIT_BREAKER_FAILURES: usize = 5;
/// Default circuit-breaker open duration before half-open probing (seconds).
pub const DEFAULT_CIRCUIT_BREAKER_TIMEOUT_SECS: u64 = 60;

/// Composed resilience policies applied in a fixed order.
#[derive(Debug, Clone)]
pub struct ResilienceStack<E> {
    /// Timeout applied as the innermost guard.
    pub(crate) timeout: TimeoutPolicy,
    /// Bulkhead limiting concurrent executions.
    pub(crate) bulkhead: BulkheadPolicy,
    /// Circuit breaker guarding failures and probing recovery.
    pub(crate) circuit_breaker: CircuitBreakerPolicy,
    /// Retry wrapper with backoff/jitter and retry predicate.
    pub(crate) retry: RetryPolicy<E>,
}

impl<E> ResilienceStack<E>
where
    E: std::error::Error + Send + Sync + 'static,
{
    /// Create a new stack builder for the given error type.
    ///
    /// ```no_run
    /// use std::time::Duration;
    /// use ninelives as your_crate;
    /// use your_crate::ResilienceStack;
    /// let stack = ResilienceStack::<std::io::Error>::new()
    ///     .timeout(Duration::from_secs(1))
    ///     .build();
    /// ```
    pub fn new() -> ResilienceStackBuilder<E> {
        ResilienceStackBuilder::new()
    }

    /// Execute an async operation through the composed resilience layers.
    ///
    /// Layer order: `Retry → CircuitBreaker → Bulkhead → Timeout → Operation`.
    /// - `T`: successful output type, must be `Send`.
    /// - `Fut`: future produced by the operation, returning `Result<T, ResilienceError<E>>`.
    /// - `Op`: invoked on each attempt to produce a fresh future; must be `FnMut() -> Fut + Send`.
    ///
    /// Errors are returned as `ResilienceError<E>`; layers may short-circuit (circuit open, bulkhead
    /// full, timeout) or propagate inner errors untouched.
    pub async fn execute<T, Fut, Op>(&self, operation: Op) -> Result<T, ResilienceError<E>>
    where
        T: Send,
        Fut: Future<Output = Result<T, ResilienceError<E>>> + Send,
        Op: FnMut() -> Fut + Send,
    {
        let op_cell = Arc::new(Mutex::new(operation));

        // Use references to avoid cloning policies on each attempt.
        let circuit_breaker = &self.circuit_breaker;
        let bulkhead = &self.bulkhead;
        let timeout = &self.timeout;

        self.retry
            .execute(|| {
                let op = op_cell.clone();

                async move {
                    circuit_breaker
                        .execute(|| {
                            let op = op.clone();

                            async move {
                                bulkhead
                                    .execute(|| {
                                        let op = op.clone();

                                        async move {
                                            timeout
                                                .execute(|| async {
                                                    let fut = {
                                                        let mut guard = op.lock().await;
                                                        (&mut *guard)()
                                                    };
                                                    fut.await
                                                })
                                                .await
                                        }
                                    })
                                    .await
                            }
                        })
                        .await
                }
            })
            .await
    }
}

impl<E> Default for ResilienceStack<E>
where
    E: std::error::Error + Send + Sync + 'static,
{
    /// Defaults: timeout 30s, bulkhead 100, circuit breaker (5 failures, 60s open), retry builder defaults.
    fn default() -> Self {
        ResilienceStackBuilder::new().build()
    }
}

/// Builder for composing resilience policies in a fixed order.
#[derive(Debug, Clone)]
pub struct ResilienceStackBuilder<E> {
    timeout: Option<TimeoutPolicy>,
    bulkhead: Option<BulkheadPolicy>,
    circuit_breaker: Option<CircuitBreakerPolicy>,
    retry: Option<RetryPolicy<E>>,
}

impl<E> ResilienceStackBuilder<E>
where
    E: std::error::Error + Send + Sync + 'static,
{
    /// Create a new builder with no policies configured yet.
    pub fn new() -> Self {
        Self { timeout: None, bulkhead: None, circuit_breaker: None, retry: None }
    }

    /// Set a timeout policy with the provided duration.
    pub fn timeout(mut self, duration: Duration) -> Self {
        self.timeout = Some(TimeoutPolicy::new(duration));
        self
    }

    /// Disable timeouts by setting an effectively infinite duration.
    pub fn no_timeout(mut self) -> Self {
        self.timeout = Some(TimeoutPolicy::new(Duration::from_secs(u64::MAX / 1000)));
        self
    }

    /// Configure a bulkhead with a maximum number of concurrent permits.
    /// Panics if `max_concurrent` is zero.
    pub fn bulkhead(mut self, max_concurrent: usize) -> Self {
        assert!(max_concurrent > 0, "max_concurrent must be > 0");
        self.bulkhead = Some(BulkheadPolicy::new(max_concurrent));
        self
    }

    /// Configure an unlimited bulkhead (no concurrency cap).
    pub fn unlimited_bulkhead(mut self) -> Self {
        self.bulkhead = Some(BulkheadPolicy::unlimited());
        self
    }

    /// Configure a circuit breaker with failure threshold and open timeout.
    /// Panics if parameters are invalid.
    pub fn circuit_breaker(mut self, failures: usize, timeout: Duration) -> Self {
        assert!(failures > 0, "failures must be > 0");
        assert!(timeout > Duration::ZERO, "timeout must be > 0");
        self.circuit_breaker = Some(CircuitBreakerPolicy::new(failures, timeout));
        self
    }

    /// Configure a circuit breaker using an explicit config.
    pub fn circuit_breaker_with_config(mut self, config: CircuitBreakerConfig) -> Self {
        self.circuit_breaker = Some(CircuitBreakerPolicy::with_config(config));
        self
    }

    /// Disable the circuit breaker layer.
    pub fn no_circuit_breaker(mut self) -> Self {
        self.circuit_breaker =
            Some(CircuitBreakerPolicy::with_config(CircuitBreakerConfig::disabled()));
        self
    }

    /// Set the retry policy to use for the outermost layer.
    pub fn retry(mut self, policy: RetryPolicy<E>) -> Self {
        self.retry = Some(policy);
        self
    }

    /// Build the stack, filling unspecified layers with crate defaults.
    pub fn build(self) -> ResilienceStack<E> {
        ResilienceStack {
            timeout: self
                .timeout
                .unwrap_or_else(|| TimeoutPolicy::new(Duration::from_secs(DEFAULT_TIMEOUT_SECS))),
            bulkhead: self
                .bulkhead
                .unwrap_or_else(|| BulkheadPolicy::new(DEFAULT_BULKHEAD_MAX_CONCURRENT)),
            circuit_breaker: self.circuit_breaker.unwrap_or_else(|| {
                CircuitBreakerPolicy::new(
                    DEFAULT_CIRCUIT_BREAKER_FAILURES,
                    Duration::from_secs(DEFAULT_CIRCUIT_BREAKER_TIMEOUT_SECS),
                )
            }),
            retry: self.retry.unwrap_or_else(|| RetryPolicy::builder().build()),
        }
    }
}

impl<E> Default for ResilienceStackBuilder<E>
where
    E: std::error::Error + Send + Sync + 'static,
{
    /// Equivalent to `ResilienceStackBuilder::new()`.
    fn default() -> Self {
        Self::new()
    }
}
