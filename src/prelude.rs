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
    control::{
        AuthMode, AuthPayload, AuthProvider, AuthRegistry, CommandContext, CommandEnvelope,
        CommandMeta, CommandService,
    },
    jitter::Jitter,
    retry::{BuildError, RetryLayer, RetryPolicy, RetryPolicyBuilder},
    telemetry::{
        BulkheadEvent, CircuitBreakerEvent, FallbackSink, LogSink, MemorySink, MulticastSink,
        NullSink, PolicyEvent, RequestOutcome, RetryEvent, StreamingSink, TelemetrySink,
        TimeoutEvent,
    },
    timeout::TimeoutLayer,
    timeout::{TimeoutError, TimeoutPolicy, MAX_TIMEOUT},
    BulkheadPolicy, ResilienceError,
};

/// Simple, ready-to-use helpers.
pub mod simple {
    use std::time::Duration;

    use crate::circuit_breaker::{CircuitBreakerConfig, CircuitBreakerLayer};
    use crate::{retry::RetryPolicy, timeout::TimeoutLayer, RetryLayer};

    /// Construct a circuit breaker layer with sensible defaults, overriding threshold and timeout.
    pub fn circuit_breaker(threshold: usize, timeout: Duration) -> CircuitBreakerLayer {
        let cfg = CircuitBreakerConfig::builder()
            .failure_threshold(threshold)
            .recovery_timeout(timeout)
            .half_open_limit(1)
            .build()
            .expect("circuit breaker config");
        CircuitBreakerLayer::new(cfg).expect("circuit breaker layer")
    }

    /// Construct a retry layer with a fixed max_attempts and default backoff/jitter.
    pub fn retry<E>(max_attempts: usize) -> RetryLayer<E>
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        RetryPolicy::<E>::builder()
            .max_attempts(max_attempts)
            .build()
            .expect("retry policy")
            .into_layer()
    }

    /// Construct a timeout layer with the provided limit.
    pub fn timeout(limit: Duration) -> TimeoutLayer {
        TimeoutLayer::new(limit).expect("timeout layer")
    }
}
