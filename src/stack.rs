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
//! use ninelives::{Backoff, Jitter, ResilienceError, ResilienceStack, RetryPolicy};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), ResilienceError<std::io::Error>> {
//!     let retry = RetryPolicy::builder()
//!         .max_attempts(3)
//!         .backoff(Backoff::exponential(Duration::from_millis(100)))
//!         .with_jitter(Jitter::full())
//!         .build()
//!         .expect("valid retry policy");
//!
//!     let stack = ResilienceStack::<std::io::Error>::new()
//!         .bulkhead(32).expect("valid bulkhead")
//!         .timeout(Duration::from_secs(2)).expect("valid timeout")
//!         .circuit_breaker(5, Duration::from_secs(30)).expect("valid breaker")
//!         .retry(retry)
//!         .build()
//!         .expect("valid stack");
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

use crate::retry::BuildError;
use crate::{
    BulkheadError, BulkheadPolicy, CircuitBreakerConfig, CircuitBreakerError, CircuitBreakerPolicy,
    ResilienceError, RetryPolicy, TimeoutError, TimeoutPolicy,
};
use std::future::Future;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StackError {
    Timeout(TimeoutError),
    Bulkhead(BulkheadError),
    CircuitBreaker(CircuitBreakerError),
    Retry(BuildError),
}

impl std::fmt::Display for StackError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StackError::Timeout(err) => write!(f, "invalid timeout configuration: {}", err),
            StackError::Bulkhead(err) => write!(f, "invalid bulkhead configuration: {}", err),
            StackError::CircuitBreaker(err) => {
                write!(f, "invalid circuit breaker configuration: {}", err)
            }
            StackError::Retry(err) => write!(f, "invalid retry configuration: {}", err),
        }
    }
}

impl std::error::Error for StackError {}
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
    /// use ninelives::ResilienceStack;
    /// let stack = ResilienceStack::<std::io::Error>::new()
    ///     .timeout(Duration::from_secs(1)).expect("valid timeout")
    ///     .build()
    ///     .expect("valid stack");
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
        Op: Fn() -> Fut + Clone + Send + Sync,
    {
        let circuit_breaker = self.circuit_breaker.clone();
        let bulkhead = self.bulkhead.clone();
        let timeout = self.timeout.clone();
        let retry = &self.retry;

        retry
            .execute(|| {
                let op_outer = operation.clone();
                let circuit_breaker = circuit_breaker.clone();
                let bulkhead = bulkhead.clone();
                let timeout = timeout.clone();

                async move {
                    circuit_breaker
                        .execute(|| {
                            let op_cb = op_outer.clone();
                            let bulkhead = bulkhead.clone();
                            let timeout = timeout.clone();
                            async move {
                                bulkhead
                                    .execute(|| {
                                        let op_bh = op_cb.clone();
                                        let timeout = timeout.clone();
                                        async move { timeout.execute(op_bh).await }
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
        ResilienceStack {
            timeout: TimeoutPolicy::new(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
                .expect("constant timeout should be valid"),
            bulkhead: BulkheadPolicy::new(DEFAULT_BULKHEAD_MAX_CONCURRENT)
                .expect("constant bulkhead should be valid"),
            circuit_breaker: CircuitBreakerPolicy::new(
                DEFAULT_CIRCUIT_BREAKER_FAILURES,
                Duration::from_secs(DEFAULT_CIRCUIT_BREAKER_TIMEOUT_SECS),
            )
            .expect("constant circuit breaker should be valid"),
            retry: RetryPolicy::builder().build().expect("default retry policy should be valid"),
        }
    }
}

/// A builder for composing timeout, bulkhead, circuit breaker, and retry policies; it validates
/// configuration early to produce clear errors.
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
    pub fn timeout(mut self, duration: Duration) -> Result<Self, StackError> {
        let timeout = TimeoutPolicy::new(duration).map_err(StackError::Timeout)?;
        self.timeout = Some(timeout);
        Ok(self)
    }

    /// Disable timeouts by setting an effectively infinite duration (u64::MAX seconds).
    pub fn no_timeout(mut self) -> Result<Self, StackError> {
        let timeout =
            TimeoutPolicy::new(crate::timeout::MAX_TIMEOUT).map_err(StackError::Timeout)?;
        self.timeout = Some(timeout);
        Ok(self)
    }

    /// Configure a bulkhead with a maximum number of concurrent permits.
    pub fn bulkhead(mut self, max_concurrent: usize) -> Result<Self, StackError> {
        let bulkhead = BulkheadPolicy::new(max_concurrent).map_err(StackError::Bulkhead)?;
        self.bulkhead = Some(bulkhead);
        Ok(self)
    }

    /// Configure an unlimited bulkhead (no concurrency cap).
    pub fn unlimited_bulkhead(mut self) -> Self {
        self.bulkhead = Some(BulkheadPolicy::unlimited());
        self
    }

    /// Configure a circuit breaker with failure threshold and open timeout.
    pub fn circuit_breaker(
        mut self,
        failures: usize,
        timeout: Duration,
    ) -> Result<Self, StackError> {
        let breaker =
            CircuitBreakerPolicy::new(failures, timeout).map_err(StackError::CircuitBreaker)?;
        self.circuit_breaker = Some(breaker);
        Ok(self)
    }

    /// Configure a circuit breaker using an explicit config.
    pub fn circuit_breaker_with_config(
        mut self,
        config: CircuitBreakerConfig,
    ) -> Result<Self, StackError> {
        let breaker =
            CircuitBreakerPolicy::with_config(config).map_err(StackError::CircuitBreaker)?;
        self.circuit_breaker = Some(breaker);
        Ok(self)
    }

    /// Disable the circuit breaker layer.
    pub fn no_circuit_breaker(mut self) -> Result<Self, StackError> {
        let breaker = CircuitBreakerPolicy::with_config(CircuitBreakerConfig::disabled())
            .map_err(StackError::CircuitBreaker)?;
        self.circuit_breaker = Some(breaker);
        Ok(self)
    }

    /// Set the retry policy to use for the outermost layer.
    pub fn retry(mut self, policy: RetryPolicy<E>) -> Self {
        self.retry = Some(policy);
        self
    }

    /// Build the stack, filling unspecified layers with crate defaults.
    pub fn build(self) -> Result<ResilienceStack<E>, StackError> {
        let timeout = match self.timeout {
            Some(t) => t,
            None => TimeoutPolicy::new(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
                .map_err(StackError::Timeout)?,
        };

        let bulkhead = match self.bulkhead {
            Some(b) => b,
            None => BulkheadPolicy::new(DEFAULT_BULKHEAD_MAX_CONCURRENT)
                .map_err(StackError::Bulkhead)?,
        };

        let circuit_breaker = match self.circuit_breaker {
            Some(cb) => cb,
            None => CircuitBreakerPolicy::new(
                DEFAULT_CIRCUIT_BREAKER_FAILURES,
                Duration::from_secs(DEFAULT_CIRCUIT_BREAKER_TIMEOUT_SECS),
            )
            .map_err(StackError::CircuitBreaker)?,
        };

        let retry = match self.retry {
            Some(r) => r,
            None => RetryPolicy::builder().build().map_err(StackError::Retry)?,
        };

        Ok(ResilienceStack { timeout, bulkhead, circuit_breaker, retry })
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

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct TestError;

    impl std::fmt::Display for TestError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "test error")
        }
    }

    impl std::error::Error for TestError {}

    #[test]
    fn default_stack_builds() {
        let stack = ResilienceStack::<TestError>::new().build();
        assert!(stack.is_ok());
    }

    #[test]
    fn bulkhead_validation_surfaces_error() {
        let err =
            ResilienceStack::<TestError>::new().bulkhead(0).expect_err("bulkhead(0) should fail");
        assert!(matches!(err, StackError::Bulkhead(_)));
    }

    #[test]
    fn timeout_validation_surfaces_error() {
        let err = ResilienceStack::<TestError>::new()
            .timeout(Duration::ZERO)
            .expect_err("zero timeout should fail");
        assert!(matches!(err, StackError::Timeout(TimeoutError::ZeroDuration)));
    }

    #[test]
    fn circuit_breaker_validation_surfaces_error() {
        let err = ResilienceStack::<TestError>::new()
            .circuit_breaker(0, Duration::from_secs(1))
            .expect_err("invalid circuit breaker should fail");
        assert!(matches!(err, StackError::CircuitBreaker(_)));
    }
}
