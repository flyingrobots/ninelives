//! Convenient re-exports for common Nine Lives types.
pub use crate::{
    adaptive::DynamicConfig,
    algebra::{CombinedLayer, FallbackLayer, ForkJoinLayer, Policy},
    backoff::{
        Backoff, BackoffError, BackoffStrategy, ConstantBackoff, ExponentialBackoff, LinearBackoff,
        MAX_BACKOFF,
    },
    bulkhead::BulkheadLayer,
    circuit_breaker::{CircuitBreakerConfig, CircuitBreakerError, CircuitBreakerLayer},
    jitter::Jitter,
    retry::{BuildError, RetryLayer, RetryPolicy, RetryPolicyBuilder},
    telemetry::{
        BulkheadEvent, CircuitBreakerEvent, FallbackSink, LogSink, MemorySink, MulticastSink,
        NullSink, PolicyEvent, RequestOutcome, RetryEvent, StreamingSink, TelemetrySink,
        TimeoutEvent,
    },
    timeout::{TimeoutError, TimeoutLayer, TimeoutPolicy, MAX_TIMEOUT},
    BulkheadPolicy, ResilienceError,
};

#[cfg(feature = "control")]
pub use crate::control::{
    AuthMode, AuthPayload, AuthProvider, AuthRegistry, CommandContext, CommandEnvelope,
    CommandMeta, CommandService,
};

/// Simple, ready-to-use helpers.
pub mod simple {
    use std::time::Duration;

    use crate::circuit_breaker::{CircuitBreakerConfig, CircuitBreakerError, CircuitBreakerLayer};
    use crate::{retry::RetryPolicy, timeout::TimeoutLayer, BuildError, RetryLayer, TimeoutError};

    /// Construct a circuit breaker layer with sensible defaults, overriding threshold and timeout.
    ///
    /// Returns `Err` if `threshold` is zero or `timeout` is zero.
    pub fn circuit_breaker(
        threshold: usize,
        timeout: Duration,
    ) -> Result<CircuitBreakerLayer, CircuitBreakerError> {
        let cfg = CircuitBreakerConfig::builder()
            .failure_threshold(threshold)
            .recovery_timeout(timeout)
            .half_open_limit(1)
            .build()?;
        CircuitBreakerLayer::new(cfg)
    }

    /// Construct a retry layer with a fixed max_attempts and default backoff/jitter.
    ///
    /// Returns `Err` if `max_attempts` is zero.
    pub fn retry<E>(max_attempts: usize) -> Result<RetryLayer<E>, BuildError>
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        Ok(RetryPolicy::<E>::builder().max_attempts(max_attempts).build()?.into_layer())
    }

    /// Construct a timeout layer with the provided limit.
    ///
    /// Returns `Err` if `limit` is zero.
    pub fn timeout(limit: Duration) -> Result<TimeoutLayer, TimeoutError> {
        TimeoutLayer::new(limit)
    }
}
pub use crate::presets;
