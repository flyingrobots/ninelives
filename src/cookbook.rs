//! Ready-to-use policy recipes (“cookbook”).
//! Each function returns a `Policy<Layer>` stack you can drop into `tower::ServiceBuilder`.
//! The goal is pragmatic defaults that are safe for production.

use std::time::Duration;

use crate::algebra::{CombinedLayer, FallbackLayer, ForkJoinLayer, Policy};
use crate::bulkhead::BulkheadLayer;
use crate::circuit_breaker::{CircuitBreakerConfig, CircuitBreakerLayer};
use crate::retry::RetryLayer;
use crate::timeout::TimeoutLayer;
use crate::{Backoff, Jitter};

/// Simple, fast retry: 3 attempts, exponential backoff starting at 50ms, full jitter.
pub fn retry_fast<E>() -> Result<Policy<RetryLayer<E>>, crate::retry::BuildError>
where
    E: std::error::Error + Send + Sync + 'static,
{
    Ok(Policy(RetryLayer::new(
        3,
        Backoff::exponential(Duration::from_millis(50)).into(),
        Jitter::full(),
        std::sync::Arc::new(|_e: &E| true),
        std::sync::Arc::new(crate::TokioSleeper::default()),
    )?))
}

/// Latency guard: 95th percentile focused timeout at 300ms.
pub fn timeout_p95() -> Result<Policy<TimeoutLayer>, crate::timeout::TimeoutError> {
    Ok(Policy(TimeoutLayer::new(Duration::from_millis(300))?))
}

/// Bulkhead for noisy neighbors: cap at `max_in_flight` with immediate rejection.
pub fn bulkhead_isolate(max_in_flight: usize) -> Result<Policy<BulkheadLayer>, crate::bulkhead::BulkheadError> {
    Ok(Policy(BulkheadLayer::new(max_in_flight)?))
}

/// Circuit breaker tuned for flapping services.
pub fn circuit_flap_guard() -> Result<Policy<CircuitBreakerLayer>, crate::circuit_breaker::CircuitBreakerError> {
    let cfg = CircuitBreakerConfig::new(5, Duration::from_secs(5), 3)?;
    Ok(Policy(CircuitBreakerLayer::new(cfg)?))
}

/// Reliable read: aggressive attempt then relaxed fallback.
/// Layout: (fast timeout + small retries) | (slow timeout + generous retries)
pub fn reliable_read<E>() -> Result<Policy<FallbackLayer<CombinedLayer<RetryLayer<E>, TimeoutLayer>, CombinedLayer<RetryLayer<E>, TimeoutLayer>>>, Box<dyn std::error::Error>>
where
    E: std::error::Error + Send + Sync + 'static,
{
    let fast = retry_fast::<E>()? + timeout_p95()?; // tighter path first
    let slow = Policy(RetryLayer::new(
        5,
        Backoff::exponential(Duration::from_millis(150)).into(),
        Jitter::full(),
        std::sync::Arc::new(|_e: &E| true),
        std::sync::Arc::new(crate::TokioSleeper::default()),
    )?) + Policy(TimeoutLayer::new(Duration::from_secs(2))?);

    Ok(fast | slow)
}

/// API guardrail: bulkhead + circuit breaker + timeout, for external calls.
pub fn api_guardrail() -> Result<Policy<CombinedLayer<CombinedLayer<TimeoutLayer, CircuitBreakerLayer>, BulkheadLayer>>, Box<dyn std::error::Error>> {
    let timeout = Policy(TimeoutLayer::new(Duration::from_secs(1))?);
    let bulkhead = bulkhead_isolate(64)?;
    let breaker = circuit_flap_guard()?;
    Ok(timeout + breaker + bulkhead)
}

/// “Four nines” read-mostly path: hedged request with short timeout and fallback path.
pub fn hedged_read<E>() -> Result<Policy<ForkJoinLayer<CombinedLayer<TimeoutLayer, RetryLayer<E>>, CombinedLayer<TimeoutLayer, RetryLayer<E>>>>, Box<dyn std::error::Error>>
where
    E: std::error::Error + Send + Sync + 'static,
{
    let fast = Policy(TimeoutLayer::new(Duration::from_millis(80))?)
        + Policy(RetryLayer::new(
            2,
            Backoff::constant(Duration::from_millis(20)).into(),
            Jitter::equal(),
            std::sync::Arc::new(|_e: &E| true),
            std::sync::Arc::new(crate::TokioSleeper::default()),
        )?);

    let steady = Policy(TimeoutLayer::new(Duration::from_millis(400))?)
        + Policy(RetryLayer::new(
            4,
            Backoff::exponential(Duration::from_millis(60)).into(),
            Jitter::full(),
            std::sync::Arc::new(|_e: &E| true),
            std::sync::Arc::new(crate::TokioSleeper::default()),
        )?);

    Ok(fast & steady)
}

/// Low-risk default: timeout + retry + bulkhead. Good starting point for most I/O.
pub fn sensible_defaults<E>(max_in_flight: usize) -> Result<Policy<SensibleStack<E>>, Box<dyn std::error::Error>>
where
    E: std::error::Error + Send + Sync + 'static,
{
    Ok(Policy(TimeoutLayer::new(Duration::from_millis(750))?)
        + Policy(RetryLayer::new(
            3,
            Backoff::exponential(Duration::from_millis(100)).into(),
            Jitter::full(),
            std::sync::Arc::new(|_e: &E| true),
            std::sync::Arc::new(crate::TokioSleeper::default()),
        )?)
        + bulkhead_isolate(max_in_flight)?)
}

type SensibleStack<E> = CombinedLayer<CombinedLayer<TimeoutLayer, RetryLayer<E>>, BulkheadLayer>;
