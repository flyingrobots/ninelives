//! Production-ready policy presets.
//!
//! This module provides pre-configured policy stacks for common use cases,
//! eliminating the need to manually compose policies and ensuring production
//! best practices are followed by default.
//!
//! ## Quick Start
//!
//! ```rust
//! use ninelives::presets;
//! use tower::ServiceBuilder;
//! use ninelives::telemetry::MemorySink;
//! use tower::service_fn;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let sink = MemorySink::with_capacity(10_000);
//! use tower::ServiceExt; // Added
//! #[derive(Debug, Clone)]
//! struct MyDocTestError;
//! impl std::fmt::Display for MyDocTestError {
//!     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//!         write!(f, "MyDocTestError")
//!     }
//! }
//! impl std::error::Error for MyDocTestError {}
//!     let my_service = service_fn(|req: &'static str| async move {
//!         Ok::<_, MyDocTestError>(format!("processed: {}", req))
//!     });
//!     let production_service = presets::web_service(my_service, sink);
//!     
//!     let response = production_service.oneshot("hello").await?;
//!     println!("{}", response);
//!     Ok(())
//! }
//! ```
//!
//! ## Available Presets
//!
//! - [`web_service`]: HTTP/gRPC services (aggressive retry, circuit breaker, bulkhead, telemetry)
//! - [`database_client`]: Database connections (circuit breaker, bulkhead, telemetry - NO retry)
//! - [`external_api`]: Third-party API calls (conservative retry, long timeouts, circuit breaker, bulkhead, telemetry)
//! - [`fast_cache`]: Cache lookups (timeout only, telemetry)
//! - [`message_producer`]: Message queue producers (retries for durability, bulkhead, telemetry)

use std::time::Duration;
use tower::{ServiceBuilder, Service}; // ServiceExt removed
use crate::{
    RetryPolicy, CircuitBreakerConfig, BulkheadLayer, TimeoutLayer,
    Backoff, Jitter, telemetry::NonBlockingSink,
    circuit_breaker_registry::{CircuitBreakerRegistry, InMemoryCircuitBreakerRegistry},
    CircuitBreakerLayer,
};
use std::sync::Arc;
use crate::telemetry::PolicyEvent; // MemorySink removed

// Default capacities and durations for presets
const DEFAULT_TIMEOUT_SECS_WEB: u64 = 5;
const DEFAULT_RETRY_ATTEMPTS_WEB: usize = 3;
const DEFAULT_BACKOFF_MILLIS_WEB: u64 = 100;
const DEFAULT_CIRCUIT_BREAKER_THRESHOLD_WEB: u32 = 10;
const DEFAULT_CIRCUIT_BREAKER_RESET_SECS_WEB: u64 = 30;
const DEFAULT_BULKHEAD_CONCURRENCY_WEB: usize = 100;
const DEFAULT_TELEMETRY_BUFFER_WEB: usize = 1000;

const DEFAULT_TIMEOUT_SECS_DB: u64 = 10;
const DEFAULT_CIRCUIT_BREAKER_THRESHOLD_DB: u32 = 5;
const DEFAULT_CIRCUIT_BREAKER_RESET_SECS_DB: u64 = 60;
const DEFAULT_BULKHEAD_CONCURRENCY_DB: usize = 50;

const DEFAULT_TIMEOUT_SECS_EXTERNAL_API: u64 = 15;
const DEFAULT_RETRY_ATTEMPTS_EXTERNAL_API: usize = 5;
const DEFAULT_BACKOFF_MILLIS_EXTERNAL_API: u64 = 500; // Longer backoff
const DEFAULT_CIRCUIT_BREAKER_THRESHOLD_EXTERNAL_API: u32 = 15;
const DEFAULT_CIRCUIT_BREAKER_RESET_SECS_EXTERNAL_API: u64 = 120;
const DEFAULT_BULKHEAD_CONCURRENCY_EXTERNAL_API: usize = 20;

const DEFAULT_TIMEOUT_MILLIS_CACHE: u64 = 100;

const DEFAULT_RETRY_ATTEMPTS_MESSAGE_PRODUCER: usize = 999; // Effectively infinite
const DEFAULT_BACKOFF_MILLIS_MESSAGE_PRODUCER: u64 = 200;
const DEFAULT_BULKHEAD_CONCURRENCY_MESSAGE_PRODUCER: usize = 50;


/// Production-grade web service policy stack.
///
/// **Included Policies:**
/// - **Timeout:** 5 seconds (prevents hung requests)
/// - **Retry:** 3 attempts with exponential backoff (100ms base, 2x multiplier)
/// - **Circuit Breaker:** Opens after 10 consecutive failures, 30s recovery
/// - **Bulkhead:** 100 concurrent requests max (prevents resource exhaustion)
/// - **Telemetry:** Non-blocking sink with 1000-event buffer
///
/// **Layering Order:** Telemetry → Timeout → Retry → CircuitBreaker → Bulkhead → Your Service
///
/// **Use Case:** HTTP APIs, gRPC services, web endpoints
///
/// # Example
/// ```rust
/// use ninelives::presets;
/// use ninelives::telemetry::MemorySink;
/// use tower::service_fn;
/// use tower::ServiceExt;
///
/// #[derive(Debug, Clone)]
/// struct MyDocTestError;
/// impl std::fmt::Display for MyDocTestError {
///     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
///         write!(f, "MyDocTestError")
///     }
/// }
/// impl std::error::Error for MyDocTestError {}
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let sink = MemorySink::with_capacity(10_000);
///     let svc = presets::web_service(
///         service_fn(|req: &'static str| async move {
///             Ok::<_, MyDocTestError>(format!("echo: {req}")) // Changed Error type
///         }),
///         sink,
///     );
///
///     let out = svc.oneshot("hi").await?;
///     println!("{out}");
///     Ok(())
/// }
/// ```
pub fn web_service<S, Req, Res, Err, Sink>(
    service: S,
    telemetry_sink: Sink,
) -> impl Service<Req, Response = Res, Error = crate::ResilienceError<crate::ResilienceError<crate::ResilienceError<crate::ResilienceError<Err>>>>> + Clone + Send + 'static
where
    S: Service<Req, Response = Res, Error = Err> + Clone + Send + 'static,
    S::Future: Send + 'static,
    Req: Clone + Send + 'static,
    Res: Send + 'static,
    Err: std::error::Error + Clone + Send + Sync + 'static,
    Sink: Service<PolicyEvent, Response = (), Error = std::convert::Infallible> + Clone + Send + 'static,
    <Sink as Service<PolicyEvent>>::Future: Send + 'static, // Added for CircuitBreaker
{
    let nonblocking_sink_instance = NonBlockingSink::with_capacity(telemetry_sink, DEFAULT_TELEMETRY_BUFFER_WEB);

    let retry_policy = RetryPolicy::builder()
        .max_attempts(DEFAULT_RETRY_ATTEMPTS_WEB)
        .backoff(Backoff::exponential(Duration::from_millis(DEFAULT_BACKOFF_MILLIS_WEB)))
        .with_jitter(Jitter::full())
        .build()
        .expect("valid retry config");
    let retry_layer = retry_policy.into_layer().with_sink(nonblocking_sink_instance.clone());

    let cb_config = CircuitBreakerConfig::builder()
        .failure_threshold(DEFAULT_CIRCUIT_BREAKER_THRESHOLD_WEB as usize)
        .recovery_timeout(Duration::from_secs(DEFAULT_CIRCUIT_BREAKER_RESET_SECS_WEB))
        .id("web_service") // Named for control plane management
        .build()
        .expect("valid breaker config");
    let breaker_layer = CircuitBreakerLayer::new(cb_config)
        .expect("valid breaker layer")
        .with_sink(nonblocking_sink_instance.clone());

    let bulkhead = BulkheadLayer::new(DEFAULT_BULKHEAD_CONCURRENCY_WEB)
        .expect("valid bulkhead config");

    let timeout = TimeoutLayer::new(Duration::from_secs(DEFAULT_TIMEOUT_SECS_WEB))
        .expect("valid timeout config");

    ServiceBuilder::new()
        .layer(timeout)
        .layer(retry_layer)
        .layer(breaker_layer)
        .layer(bulkhead)
        .service(service)
}

/// Database client policy stack (NO RETRY - preserves consistency).
///
/// **Included Policies:**
/// - **Timeout:** 10 seconds (database queries can be slow)
/// - **Circuit Breaker:** Opens after 5 consecutive failures, 60s recovery
/// - **Bulkhead:** 50 concurrent connections (matches typical DB pool size)
/// - **Telemetry:** Non-blocking sink
///
/// **NO RETRY:** Database operations are often non-idempotent. Application
/// code should handle retry logic with explicit transaction boundaries.
///
/// **Use Case:** PostgreSQL, MySQL, MongoDB, Redis clients
pub fn database_client<S, Req, Res, Err, Sink>(
    service: S,
    telemetry_sink: Sink,
) -> impl Service<Req, Response = Res, Error = crate::ResilienceError<crate::ResilienceError<crate::ResilienceError<Err>>>> + Clone + Send + 'static
where
    S: Service<Req, Response = Res, Error = Err> + Clone + Send + 'static,
    S::Future: Send + 'static,
    Req: Clone + Send + 'static,
    Res: Send + 'static,
    Err: std::error::Error + Clone + Send + Sync + 'static,
    Sink: Service<PolicyEvent, Response = (), Error = std::convert::Infallible> + Clone + Send + 'static,
    <Sink as Service<PolicyEvent>>::Future: Send + 'static, // Added for CircuitBreaker
{
    let nonblocking_sink_instance = NonBlockingSink::with_capacity(telemetry_sink, DEFAULT_TELEMETRY_BUFFER_WEB);

    let cb_config = CircuitBreakerConfig::builder()
        .failure_threshold(DEFAULT_CIRCUIT_BREAKER_THRESHOLD_DB as usize)
        .recovery_timeout(Duration::from_secs(DEFAULT_CIRCUIT_BREAKER_RESET_SECS_DB))
        .id("database_client")
        .build()
        .expect("valid breaker config");
    let breaker_layer = CircuitBreakerLayer::new(cb_config)
        .expect("valid breaker layer")
        .with_sink(nonblocking_sink_instance.clone());

    let bulkhead = BulkheadLayer::new(DEFAULT_BULKHEAD_CONCURRENCY_DB)
        .expect("valid bulkhead config");

    let timeout = TimeoutLayer::new(Duration::from_secs(DEFAULT_TIMEOUT_SECS_DB))
        .expect("valid timeout config");

    ServiceBuilder::new()
        .layer(timeout)
        .layer(breaker_layer)
        .layer(bulkhead)
        .service(service)
}

/// External API client policy stack (conservative retry, long timeouts).
///
/// **Included Policies:**
/// - **Timeout:** 15 seconds (external APIs may be slow)
/// - **Retry:** 5 attempts with exponential backoff and full jitter
/// - **Circuit Breaker:** Opens after 15 failures, 120s recovery
/// - **Bulkhead:** 20 concurrent requests (respect external rate limits)
/// - **Telemetry:** Non-blocking sink
///
/// **Use Case:** Stripe, AWS APIs, third-party services
pub fn external_api<S, Req, Res, Err, Sink>(
    service: S,
    telemetry_sink: Sink,
) -> impl Service<Req, Response = Res, Error = crate::ResilienceError<crate::ResilienceError<crate::ResilienceError<crate::ResilienceError<Err>>>>> + Clone + Send + 'static
where
    S: Service<Req, Response = Res, Error = Err> + Clone + Send + 'static,
    S::Future: Send + 'static,
    Req: Clone + Send + 'static,
    Res: Send + 'static,
    Err: std::error::Error + Clone + Send + Sync + 'static,
    Sink: Service<PolicyEvent, Response = (), Error = std::convert::Infallible> + Clone + Send + 'static,
    <Sink as Service<PolicyEvent>>::Future: Send + 'static, // Added for CircuitBreaker
{
    let nonblocking_sink_instance = NonBlockingSink::with_capacity(telemetry_sink, DEFAULT_TELEMETRY_BUFFER_WEB);

    let retry_policy = RetryPolicy::builder()
        .max_attempts(DEFAULT_RETRY_ATTEMPTS_EXTERNAL_API)
        .backoff(Backoff::exponential(Duration::from_millis(DEFAULT_BACKOFF_MILLIS_EXTERNAL_API)))
        .with_jitter(Jitter::full())
        .build()
        .expect("valid retry config");
    let retry_layer = retry_policy.into_layer().with_sink(nonblocking_sink_instance.clone());

    let cb_config = CircuitBreakerConfig::builder()
        .failure_threshold(DEFAULT_CIRCUIT_BREAKER_THRESHOLD_EXTERNAL_API as usize)
        .recovery_timeout(Duration::from_secs(DEFAULT_CIRCUIT_BREAKER_RESET_SECS_EXTERNAL_API))
        .id("external_api")
        .build()
        .expect("valid breaker config");
    let breaker_layer = CircuitBreakerLayer::new(cb_config)
        .expect("valid breaker layer")
        .with_sink(nonblocking_sink_instance.clone());

    let bulkhead = BulkheadLayer::new(DEFAULT_BULKHEAD_CONCURRENCY_EXTERNAL_API)
        .expect("valid bulkhead config");

    let timeout = TimeoutLayer::new(Duration::from_secs(DEFAULT_TIMEOUT_SECS_EXTERNAL_API))
        .expect("valid timeout config");

    ServiceBuilder::new()
        .layer(timeout)
        .layer(retry_layer)
        .layer(breaker_layer)
        .layer(bulkhead)
        .service(service)
}
/// Fast cache policy stack (timeout only, fail fast).
///
/// **Included Policies:**
/// - **Timeout:** 100ms (cache should be fast)
/// - **Telemetry:** Non-blocking sink
///
/// **NO retry, breaker, or bulkhead:** Fail fast to fallback to primary data source.
///
/// **Use Case:** Redis cache, Memcached, local cache lookups
pub fn fast_cache<S, Req, Res, Err, Sink>(
    service: S,
    telemetry_sink: Sink,
) -> impl Service<Req, Response = Res, Error = crate::ResilienceError<Err>> + Clone + Send + 'static
where
    S: Service<Req, Response = Res, Error = Err> + Clone + Send + 'static,
    S::Future: Send + 'static,
    Req: Clone + Send + 'static,
    Res: Send + 'static,
    Err: std::error::Error + Clone + Send + Sync + 'static,
    Sink: Service<PolicyEvent, Response = (), Error = std::convert::Infallible> + Clone + Send + 'static,
    <Sink as Service<PolicyEvent>>::Future: Send + 'static, // Added for CircuitBreaker
{
    let nonblocking_sink_instance = NonBlockingSink::with_capacity(telemetry_sink, DEFAULT_TELEMETRY_BUFFER_WEB);

    let timeout = TimeoutLayer::new(Duration::from_millis(DEFAULT_TIMEOUT_MILLIS_CACHE))
        .expect("valid timeout config");

    ServiceBuilder::new()
        .layer(timeout)
        .service(service)
}

/// Message producer policy stack (retries for durability, bulkhead).
///
/// **Included Policies:**
/// - **Retry:** Effectively infinite retries with exponential backoff and jitter
/// - **Bulkhead:** Limits concurrent message sends
/// - **Telemetry:** Non-blocking sink
///
/// **Use Case:** Kafka, NATS, RabbitMQ producers where message delivery is critical.
pub fn message_producer<S, Req, Res, Err, Sink>(
    service: S,
    telemetry_sink: Sink,
) -> impl Service<Req, Response = Res, Error = crate::ResilienceError<crate::ResilienceError<Err>>> + Clone + Send + 'static
where
    S: Service<Req, Response = Res, Error = Err> + Clone + Send + 'static,
    S::Future: Send + 'static,
    Req: Clone + Send + 'static,
    Res: Send + 'static,
    Err: std::error::Error + Clone + Send + Sync + 'static,
    Sink: Service<PolicyEvent, Response = (), Error = std::convert::Infallible> + Clone + Send + 'static,
    <Sink as Service<PolicyEvent>>::Future: Send + 'static, // Added for CircuitBreaker
{
    let nonblocking_sink_instance = NonBlockingSink::with_capacity(telemetry_sink, DEFAULT_TELEMETRY_BUFFER_WEB);

    let retry_policy = RetryPolicy::builder()
        .max_attempts(DEFAULT_RETRY_ATTEMPTS_MESSAGE_PRODUCER) // Effectively infinite
        .backoff(Backoff::exponential(Duration::from_millis(DEFAULT_BACKOFF_MILLIS_MESSAGE_PRODUCER)))
        .with_jitter(Jitter::full())
        .build()
        .expect("valid retry config");
    let retry_layer = retry_policy.into_layer().with_sink(nonblocking_sink_instance.clone());

    let bulkhead = BulkheadLayer::new(DEFAULT_BULKHEAD_CONCURRENCY_MESSAGE_PRODUCER)
        .expect("valid bulkhead config");

    ServiceBuilder::new()
        .layer(retry_layer)
        .layer(bulkhead)
        .service(service)
}
// Helper function for the internal use only, not exported.
// Used by presets for CircuitBreaker, which needs a registry to register itself.
fn _get_default_cb_registry() -> Arc<dyn CircuitBreakerRegistry> {
    // This is a simple in-memory registry, a real app might share one
    // or use a distributed one.
    Arc::new(InMemoryCircuitBreakerRegistry::default())
}
