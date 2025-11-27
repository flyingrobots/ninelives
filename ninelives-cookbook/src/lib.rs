//! Ready-to-use policy recipes (“cookbook”) for ninelives.
//! Each function returns a `Policy<Layer>` stack you can drop into `tower::ServiceBuilder`.
//! The goal is pragmatic defaults that are safe for production.
//!
//! **Ladder:**
//! - Simple: [`retry_fast`], [`timeout_p95`], [`bulkhead_isolate`]
//! - Intermediate: [`api_guardrail`]
//! - Advanced: [`reliable_read`]
//! - Tricky: [`hedged_read`]
//! - God tier: [`hedged_then_fallback`]
//! - Starter pack: [`sensible_defaults`]

use std::time::Duration;

use ninelives::algebra::{CombinedLayer, FallbackLayer, ForkJoinLayer, Policy};
use ninelives::bulkhead::BulkheadLayer;
use ninelives::circuit_breaker::{CircuitBreakerConfig, CircuitBreakerLayer};
use ninelives::retry::RetryLayer;
use ninelives::timeout::TimeoutLayer;
use ninelives::{Backoff, Jitter};

/// Simple, fast retry: 3 attempts, exponential backoff starting at 50ms, full jitter.
pub fn retry_fast<E>() -> Result<Policy<RetryLayer<E>>, ninelives::retry::BuildError>
where
    E: std::error::Error + Send + Sync + 'static,
{
    Ok(Policy(RetryLayer::new(
        3,
        Backoff::exponential(Duration::from_millis(50)).into(),
        Jitter::full(),
        std::sync::Arc::new(|_e: &E| true),
        std::sync::Arc::new(ninelives::TokioSleeper::default()),
    )?))
}

/// Latency guard: 95th percentile focused timeout at 300ms.
pub fn timeout_p95() -> Result<Policy<TimeoutLayer>, ninelives::timeout::TimeoutError> {
    Ok(Policy(TimeoutLayer::new(Duration::from_millis(300))?))
}

/// Bulkhead for noisy neighbors: cap at `max_in_flight` with immediate rejection.
pub fn bulkhead_isolate(
    max_in_flight: usize,
) -> Result<Policy<BulkheadLayer>, ninelives::bulkhead::BulkheadError> {
    Ok(Policy(BulkheadLayer::new(max_in_flight)?))
}

/// Circuit breaker tuned for flapping services.
pub fn circuit_flap_guard(
) -> Result<Policy<CircuitBreakerLayer>, ninelives::circuit_breaker::CircuitBreakerError> {
    let cfg = CircuitBreakerConfig::new(5, Duration::from_secs(5), 3)?;
    Ok(Policy(CircuitBreakerLayer::new(cfg)?))
}

/// Reliable read: aggressive attempt then relaxed fallback.
/// Layout: (fast timeout + small retries) | (slow timeout + generous retries)
pub fn reliable_read<E>() -> Result<
    Policy<
        FallbackLayer<
            CombinedLayer<RetryLayer<E>, TimeoutLayer>,
            CombinedLayer<RetryLayer<E>, TimeoutLayer>,
        >,
    >,
    Box<dyn std::error::Error>,
>
where
    E: std::error::Error + Send + Sync + 'static,
{
    let fast = retry_fast::<E>()? + timeout_p95()?; // tighter path first
    let slow = Policy(RetryLayer::new(
        5,
        Backoff::exponential(Duration::from_millis(150)).into(),
        Jitter::full(),
        std::sync::Arc::new(|_e: &E| true),
        std::sync::Arc::new(ninelives::TokioSleeper::default()),
    )?) + Policy(TimeoutLayer::new(Duration::from_secs(2))?);

    Ok(fast | slow)
}

/// API guardrail: bulkhead + circuit breaker + timeout, for external calls.
pub fn api_guardrail() -> Result<
    Policy<CombinedLayer<CombinedLayer<TimeoutLayer, CircuitBreakerLayer>, BulkheadLayer>>,
    Box<dyn std::error::Error>,
> {
    let timeout = Policy(TimeoutLayer::new(Duration::from_secs(1))?);
    let bulkhead = bulkhead_isolate(64)?;
    let breaker = circuit_flap_guard()?;
    Ok(timeout + breaker + bulkhead)
}

/// “Four nines” read-mostly path: hedged request with short timeout and fallback path.
pub fn hedged_read<E>() -> Result<
    Policy<
        ForkJoinLayer<
            CombinedLayer<TimeoutLayer, RetryLayer<E>>,
            CombinedLayer<TimeoutLayer, RetryLayer<E>>,
        >,
    >,
    Box<dyn std::error::Error>,
>
where
    E: std::error::Error + Send + Sync + 'static,
{
    let fast = Policy(TimeoutLayer::new(Duration::from_millis(80))?)
        + Policy(RetryLayer::new(
            2,
            Backoff::constant(Duration::from_millis(20)).into(),
            Jitter::equal(),
            std::sync::Arc::new(|_e: &E| true),
            std::sync::Arc::new(ninelives::TokioSleeper::default()),
        )?);

    let steady = Policy(TimeoutLayer::new(Duration::from_millis(400))?)
        + Policy(RetryLayer::new(
            4,
            Backoff::exponential(Duration::from_millis(60)).into(),
            Jitter::full(),
            std::sync::Arc::new(|_e: &E| true),
            std::sync::Arc::new(ninelives::TokioSleeper::default()),
        )?);

    Ok(fast & steady)
}

/// Low-risk default: timeout + retry + bulkhead. Good starting point for most I/O.
pub fn sensible_defaults<E>(
    max_in_flight: usize,
) -> Result<Policy<SensibleStack<E>>, Box<dyn std::error::Error>>
where
    E: std::error::Error + Send + Sync + 'static,
{
    Ok(Policy(TimeoutLayer::new(Duration::from_millis(750))?)
        + Policy(RetryLayer::new(
            3,
            Backoff::exponential(Duration::from_millis(100)).into(),
            Jitter::full(),
            std::sync::Arc::new(|_e: &E| true),
            std::sync::Arc::new(ninelives::TokioSleeper::default()),
        )?)
        + bulkhead_isolate(max_in_flight)?)
}

type SensibleStack<E> = CombinedLayer<CombinedLayer<TimeoutLayer, RetryLayer<E>>, BulkheadLayer>;

/// Hedged first, then fall back to a sturdier stack.
/// Layout: (fast hedge of two stacks) | (slow but sturdy stack)
pub fn hedged_then_fallback<E>(
) -> Result<Policy<FallbackLayer<Hedge<E>, Sturdy<E>>>, Box<dyn std::error::Error>>
where
    E: std::error::Error + Send + Sync + 'static,
{
    let hedge = hedged_read::<E>()?; // fast twin paths

    let sturdy = Policy(TimeoutLayer::new(Duration::from_secs(2))?)
        + Policy(CircuitBreakerLayer::new(CircuitBreakerConfig::new(
            8,
            Duration::from_secs(10),
            3,
        )?)?)
        + Policy(RetryLayer::new(
            4,
            Backoff::exponential(Duration::from_millis(120)).into(),
            Jitter::full(),
            std::sync::Arc::new(|_e: &E| true),
            std::sync::Arc::new(ninelives::TokioSleeper::default()),
        )?);

    Ok(hedge | sturdy)
}

type Hedge<E> = ForkJoinLayer<
    CombinedLayer<TimeoutLayer, RetryLayer<E>>,
    CombinedLayer<TimeoutLayer, RetryLayer<E>>,
>;
type Sturdy<E> = CombinedLayer<CombinedLayer<TimeoutLayer, CircuitBreakerLayer>, RetryLayer<E>>;
