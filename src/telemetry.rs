//! Telemetry and observability for Nine Lives policies.
//!
//! This module provides the event system that enables all policies to emit
//! structured telemetry. Events flow through `TelemetrySink` implementations
//! which can log, aggregate, or forward events to external systems.
//!
//! # Event Types
//!
//! Each policy type emits specific events:
//!
//! - **Retry**: `RetryAttempt`, `RetryExhausted`
//! - **Circuit Breaker**: `CircuitOpened`, `CircuitClosed`, `CircuitHalfOpen`
//! - **Bulkhead**: `BulkheadAcquired`, `BulkheadRejected`
//! - **Timeout**: `TimeoutOccurred`
//! - **All policies**: `RequestSuccess`, `RequestFailure`
//!
//! # Telemetry Sinks
//!
//! The `TelemetrySink` trait defines how events are consumed. It's implemented
//! as a `tower::Service<PolicyEvent>` for composability.
//!
//! ```rust
//! use ninelives::telemetry::{PolicyEvent, RetryEvent, RequestOutcome};
//! use std::time::Duration;
//!
//! // Events emitted during policy execution
//! let retry_attempt = PolicyEvent::Retry(RetryEvent::Attempt {
//!     attempt: 1,
//!     delay: Duration::from_millis(100),
//! });
//!
//! let request_success = PolicyEvent::Request(RequestOutcome::Success {
//!     duration: Duration::from_millis(50),
//! });
//! ```

use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// A telemetry sink that consumes policy events.
///
/// This is a type alias for a `tower::Service` that processes `PolicyEvent`s.
/// Sinks can be composed using standard tower combinators, and multiple sinks
/// can be combined to create complex telemetry pipelines.
///
/// # Implementing a Custom Sink
///
/// ```rust
/// use ninelives::telemetry::{TelemetrySink, PolicyEvent};
/// use tower::Service;
/// use std::task::{Context, Poll};
/// use std::pin::Pin;
/// use std::future::Future;
///
/// #[derive(Clone)]
/// struct MySink;
///
/// impl Service<PolicyEvent> for MySink {
///     type Response = ();
///     type Error = std::convert::Infallible;
///     type Future = Pin<Box<dyn Future<Output = Result<(), Self::Error>> + Send>>;
///
///     fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
///         Poll::Ready(Ok(()))
///     }
///
///     fn call(&mut self, event: PolicyEvent) -> Self::Future {
///         println!("Received event: {}", event);
///         Box::pin(async { Ok(()) })
///     }
/// }
/// ```
pub trait TelemetrySink:
    tower::Service<PolicyEvent, Response = (), Error = Self::SinkError> + Clone + Send + 'static
{
    /// The error type for this sink.
    type SinkError: std::error::Error + Send + 'static;
}

/// Best-effort emit helper that honors `poll_ready` and swallows errors.
///
/// We keep telemetry non-blocking for policy hot paths: if a sink is not ready
/// or returns an error, we simply drop the event.
pub async fn emit_best_effort<S>(sink: S, event: PolicyEvent)
where
    S: tower::Service<PolicyEvent, Response = ()> + Send + Clone + 'static,
    S::Error: std::error::Error + Send + 'static,
    S::Future: Send + 'static,
{
    use tower::ServiceExt;

    if let Ok(mut ready_sink) = sink.ready_oneshot().await {
        let _ = ready_sink.call(event).await;
    }
}

/// Policy events emitted during execution.
///
/// All Nine Lives policies emit structured events that describe their behavior.
/// These events can be collected, aggregated, and used for observability,
/// monitoring, or autonomous control.
#[derive(Debug, Clone, PartialEq)]
pub enum PolicyEvent {
    /// Retry policy events
    Retry(RetryEvent),
    /// Circuit breaker events
    CircuitBreaker(CircuitBreakerEvent),
    /// Bulkhead events
    Bulkhead(BulkheadEvent),
    /// Timeout events
    Timeout(TimeoutEvent),
    /// Request outcome events (emitted by all policies)
    Request(RequestOutcome),
}

/// Events emitted by retry policies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RetryEvent {
    /// A retry attempt is about to be made.
    ///
    /// Emitted before sleeping and retrying a failed request.
    Attempt {
        /// The attempt number (1-indexed)
        attempt: usize,
        /// The backoff delay before this retry
        delay: Duration,
    },
    /// All retry attempts have been exhausted.
    ///
    /// Emitted when the maximum number of retries is reached
    /// and the request still fails.
    Exhausted {
        /// Total number of attempts made
        total_attempts: usize,
        /// Total time spent retrying
        total_duration: Duration,
    },
}

/// Events emitted by circuit breaker policies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitBreakerEvent {
    /// Circuit transitioned to open state.
    ///
    /// Subsequent requests will be rejected immediately without
    /// being forwarded to the inner service.
    Opened {
        /// Number of consecutive failures that triggered the open
        failure_count: usize,
    },
    /// Circuit transitioned to half-open state.
    ///
    /// A limited number of test requests will be allowed through
    /// to determine if the inner service has recovered.
    HalfOpen,
    /// Circuit transitioned to closed state.
    ///
    /// Normal operation resumes - all requests are forwarded.
    Closed,
}

/// Events emitted by bulkhead policies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BulkheadEvent {
    /// A request successfully acquired a bulkhead permit.
    ///
    /// The request will proceed to the inner service.
    Acquired {
        /// Current number of active requests
        active_count: usize,
        /// Maximum concurrency limit
        max_concurrency: usize,
    },
    /// A request was rejected due to bulkhead saturation.
    ///
    /// All available permits are in use.
    Rejected {
        /// Current number of active requests
        active_count: usize,
        /// Maximum concurrency limit
        max_concurrency: usize,
    },
}

/// Events emitted by timeout policies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeoutEvent {
    /// A request exceeded the timeout duration.
    ///
    /// The request was cancelled and an error returned.
    Occurred {
        /// The timeout duration that was exceeded
        timeout: Duration,
    },
}

/// Request outcome events emitted by all policies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestOutcome {
    /// Request completed successfully.
    Success {
        /// Time taken to complete the request
        duration: Duration,
    },
    /// Request failed with an error.
    Failure {
        /// Time taken before failure
        duration: Duration,
    },
}

impl fmt::Display for PolicyEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PolicyEvent::Retry(event) => write!(f, "Retry::{}", event),
            PolicyEvent::CircuitBreaker(event) => write!(f, "CircuitBreaker::{}", event),
            PolicyEvent::Bulkhead(event) => write!(f, "Bulkhead::{}", event),
            PolicyEvent::Timeout(event) => write!(f, "Timeout::{}", event),
            PolicyEvent::Request(event) => write!(f, "Request::{}", event),
        }
    }
}

impl fmt::Display for RetryEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RetryEvent::Attempt { attempt, delay } => {
                write!(f, "Attempt(#{}, delay={:?})", attempt, delay)
            }
            RetryEvent::Exhausted { total_attempts, total_duration } => {
                write!(f, "Exhausted(attempts={}, duration={:?})", total_attempts, total_duration)
            }
        }
    }
}

impl fmt::Display for CircuitBreakerEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CircuitBreakerEvent::Opened { failure_count } => {
                write!(f, "Opened(failures={})", failure_count)
            }
            CircuitBreakerEvent::HalfOpen => write!(f, "HalfOpen"),
            CircuitBreakerEvent::Closed => write!(f, "Closed"),
        }
    }
}

impl fmt::Display for BulkheadEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BulkheadEvent::Acquired { active_count, max_concurrency } => {
                write!(f, "Acquired({}/{})", active_count, max_concurrency)
            }
            BulkheadEvent::Rejected { active_count, max_concurrency } => {
                write!(f, "Rejected({}/{})", active_count, max_concurrency)
            }
        }
    }
}

impl fmt::Display for TimeoutEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TimeoutEvent::Occurred { timeout } => write!(f, "Occurred(timeout={:?})", timeout),
        }
    }
}

impl fmt::Display for RequestOutcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RequestOutcome::Success { duration } => write!(f, "Success(duration={:?})", duration),
            RequestOutcome::Failure { duration } => write!(f, "Failure(duration={:?})", duration),
        }
    }
}

// ============================================================================
// Built-in Telemetry Sinks
// ============================================================================

use std::convert::Infallible;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use tower::Service;

/// A no-op telemetry sink that discards all events.
///
/// Useful for testing or when telemetry is disabled.
///
/// # Example
///
/// ```rust
/// use ninelives::telemetry::{NullSink, PolicyEvent, RetryEvent};
/// use tower::Service;
/// use std::time::Duration;
///
/// # #[tokio::main]
/// # async fn main() {
/// let mut sink = NullSink;
/// let event = PolicyEvent::Retry(RetryEvent::Attempt {
///     attempt: 1,
///     delay: Duration::from_millis(100),
/// });
///
/// // Event is silently discarded
/// let _ = sink.call(event).await;
/// # }
/// ```
#[derive(Clone, Debug, Default)]
pub struct NullSink;

impl Service<PolicyEvent> for NullSink {
    type Response = ();
    type Error = Infallible;
    type Future = Pin<Box<dyn std::future::Future<Output = Result<(), Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, _event: PolicyEvent) -> Self::Future {
        Box::pin(async { Ok(()) })
    }
}

impl TelemetrySink for NullSink {
    type SinkError = Infallible;
}

/// A telemetry sink that logs events using the `tracing` crate.
///
/// Events are logged at INFO level with structured fields.
///
/// # Example
///
/// ```rust
/// use ninelives::telemetry::{LogSink, PolicyEvent, CircuitBreakerEvent};
/// use tower::Service;
///
/// # #[tokio::main]
/// # async fn main() {
/// let mut sink = LogSink;
/// let event = PolicyEvent::CircuitBreaker(CircuitBreakerEvent::Opened {
///     failure_count: 5,
/// });
///
/// // Logs: "policy_event{event=CircuitBreaker::Opened(failures=5)}"
/// let _ = sink.call(event).await;
/// # }
/// ```
#[derive(Clone, Debug, Default)]
pub struct LogSink;

impl Service<PolicyEvent> for LogSink {
    type Response = ();
    type Error = Infallible;
    type Future = Pin<Box<dyn std::future::Future<Output = Result<(), Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, event: PolicyEvent) -> Self::Future {
        tracing::info!(event = %event, "policy_event");
        Box::pin(async { Ok(()) })
    }
}

impl TelemetrySink for LogSink {
    type SinkError = Infallible;
}

/// A telemetry sink that stores events in memory.
///
/// Useful for testing and debugging. Events are stored in a `Vec` protected
/// by a `Mutex`.
///
/// # Example
///
/// ```rust
/// use ninelives::telemetry::{MemorySink, PolicyEvent, TimeoutEvent};
/// use tower::Service;
/// use std::time::Duration;
///
/// # #[tokio::main]
/// # async fn main() {
/// let mut sink = MemorySink::new();
/// let event = PolicyEvent::Timeout(TimeoutEvent::Occurred {
///     timeout: Duration::from_secs(1),
/// });
///
/// sink.call(event.clone()).await.unwrap();
///
/// let events = sink.events();
/// assert_eq!(events.len(), 1);
/// assert_eq!(events[0], event);
/// # }
/// ```
#[derive(Clone, Debug)]
pub struct MemorySink {
    events: Arc<Mutex<Vec<PolicyEvent>>>,
    capacity: usize,
    evicted: Arc<AtomicU64>,
}

impl MemorySink {
    /// Creates a bounded memory sink (default cap: 10,000).
    /// Oldest events are evicted when capacity is exceeded.
    pub fn new() -> Self {
        Self::with_capacity(10_000)
    }

    /// Creates a bounded memory sink with explicit capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            events: Arc::new(Mutex::new(Vec::new())),
            capacity: capacity.max(1),
            evicted: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Creates an unbounded memory sink. Dangerous in production.
    pub fn unbounded() -> Self {
        Self {
            events: Arc::new(Mutex::new(Vec::new())),
            capacity: usize::MAX,
            evicted: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Returns a snapshot of all events received so far.
    pub fn events(&self) -> Vec<PolicyEvent> {
        self.events.lock().unwrap().clone()
    }

    /// Clears all stored events.
    pub fn clear(&self) {
        self.events.lock().unwrap().clear();
    }

    /// Returns the number of events stored.
    pub fn len(&self) -> usize {
        self.events.lock().unwrap().len()
    }

    /// Returns true if no events are stored.
    pub fn is_empty(&self) -> bool {
        self.events.lock().unwrap().is_empty()
    }

    /// Returns the configured capacity.
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Returns the number of evicted events.
    pub fn evicted(&self) -> u64 {
        self.evicted.load(Ordering::Relaxed)
    }
}

impl Default for MemorySink {
    fn default() -> Self {
        Self::new()
    }
}

impl Service<PolicyEvent> for MemorySink {
    type Response = ();
    type Error = Infallible;
    type Future = Pin<Box<dyn std::future::Future<Output = Result<(), Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, event: PolicyEvent) -> Self::Future {
        let mut guard = self.events.lock().unwrap();
        if guard.len() >= self.capacity {
            guard.remove(0);
            self.evicted.fetch_add(1, Ordering::Relaxed);
        }
        guard.push(event);
        Box::pin(async { Ok(()) })
    }
}

impl TelemetrySink for MemorySink {
    type SinkError = Infallible;
}

/// A streaming telemetry sink that broadcasts events to multiple subscribers.
///
/// Uses `tokio::sync::broadcast` to publish events to all active receivers.
/// Receivers that fall behind will miss events (the channel has bounded capacity).
///
/// # Example
///
/// ```rust
/// use ninelives::telemetry::{StreamingSink, PolicyEvent, RetryEvent};
/// use tower::Service;
/// use std::time::Duration;
///
/// # #[tokio::main]
/// # async fn main() {
/// let sink = StreamingSink::new(100); // 100 event buffer
/// let mut receiver = sink.subscribe();
///
/// let mut sink_clone = sink.clone();
/// let event = PolicyEvent::Retry(RetryEvent::Attempt {
///     attempt: 1,
///     delay: Duration::from_millis(100),
/// });
///
/// // Send event through sink
/// sink_clone.call(event.clone()).await.unwrap();
///
/// // Receive from subscriber
/// let received = receiver.recv().await.unwrap();
/// assert_eq!(received, event);
/// # }
/// ```
#[derive(Clone, Debug)]
pub struct StreamingSink {
    sender: Arc<tokio::sync::broadcast::Sender<PolicyEvent>>,
    dropped: Arc<AtomicU64>,
    last_drop_ns: Arc<AtomicU64>,
}

impl StreamingSink {
    /// Creates a new streaming sink with the specified buffer capacity.
    ///
    /// When the buffer is full, the oldest events will be dropped for slow receivers.
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = tokio::sync::broadcast::channel(capacity);
        Self {
            sender: Arc::new(sender),
            dropped: Arc::new(AtomicU64::new(0)),
            last_drop_ns: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Subscribe to receive events from this sink.
    ///
    /// Returns a receiver that will get all future events sent through this sink.
    /// Multiple receivers can subscribe simultaneously.
    pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<PolicyEvent> {
        self.sender.subscribe()
    }

    /// Returns the number of active subscribers.
    pub fn receiver_count(&self) -> usize {
        self.sender.receiver_count()
    }

    /// Returns number of events dropped due to slow subscribers.
    pub fn dropped_count(&self) -> u64 {
        self.dropped.load(Ordering::Relaxed)
    }

    /// Timestamp of last drop, if any.
    pub fn last_drop(&self) -> Option<SystemTime> {
        match self.last_drop_ns.load(Ordering::Relaxed) {
            0 => None,
            ns => UNIX_EPOCH.checked_add(Duration::from_nanos(ns)),
        }
    }
}

impl Service<PolicyEvent> for StreamingSink {
    type Response = ();
    type Error = Infallible;
    type Future = Pin<Box<dyn std::future::Future<Output = Result<(), Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, event: PolicyEvent) -> Self::Future {
        // Send is best-effort - ignore if no receivers
        if let Err(_e) = self.sender.send(event) {
            // Receiver lagged or none connected
            self.dropped.fetch_add(1, Ordering::Relaxed);
            self.last_drop_ns.store(
                SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos() as u64,
                Ordering::Relaxed,
            );
        }
        Box::pin(async { Ok(()) })
    }
}

impl TelemetrySink for StreamingSink {
    type SinkError = Infallible;
}

// ============================================================================
// Non-blocking sink wrapper
// ============================================================================

/// Offloads telemetry emission to a bounded channel and worker task.
/// Keeps policy hot paths from awaiting slow sinks.
#[derive(Clone)]
pub struct NonBlockingSink<S> {
    tx: tokio::sync::mpsc::Sender<PolicyEvent>,
    dropped: Arc<AtomicU64>,
    _sink: Arc<tokio::sync::Mutex<S>>, // keep sink alive
}

impl<S> NonBlockingSink<S>
where
    S: tower::Service<PolicyEvent, Response = ()> + Send + Clone + 'static,
    S::Error: std::error::Error + Send + 'static,
    S::Future: Send + 'static,
{
    /// Create a new non-blocking wrapper with bounded queue and background worker.
    pub fn with_capacity(sink: S, capacity: usize) -> Self {
        let (tx, mut rx) = tokio::sync::mpsc::channel(capacity);
        let dropped = Arc::new(AtomicU64::new(0));
        let dropped_clone = dropped.clone();
        let sink_arc = Arc::new(tokio::sync::Mutex::new(sink));
        let sink_worker = sink_arc.clone();

        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                use tower::ServiceExt;
                let mut guard = sink_worker.lock().await;
                if let Ok(ready) = guard.ready().await {
                    let _ = ready.call(event).await;
                }
            }
        });

        Self { tx, dropped: dropped_clone, _sink: sink_arc }
    }

    /// How many events were dropped because the queue was full.
    pub fn dropped(&self) -> u64 {
        self.dropped.load(Ordering::Relaxed)
    }
}

impl<S> tower::Service<PolicyEvent> for NonBlockingSink<S>
where
    S: tower::Service<PolicyEvent, Response = ()> + Send + Clone + 'static,
    S::Error: std::error::Error + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = ();
    type Error = Infallible;
    type Future = Pin<Box<dyn std::future::Future<Output = Result<(), Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, event: PolicyEvent) -> Self::Future {
        if self.tx.try_send(event).is_err() {
            self.dropped.fetch_add(1, Ordering::Relaxed);
        }
        Box::pin(async { Ok(()) })
    }
}

impl<S> TelemetrySink for NonBlockingSink<S>
where
    S: tower::Service<PolicyEvent, Response = ()> + Send + Clone + 'static,
    S::Error: std::error::Error + Send + 'static,
    S::Future: Send + 'static,
{
    type SinkError = Infallible;
}

// ============================================================================
// Telemetry Sink Composition
// ============================================================================

/// Error type for composed telemetry sinks.
#[derive(Debug)]
pub struct ComposedSinkError(Box<dyn std::error::Error + Send + Sync>);

impl std::fmt::Display for ComposedSinkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "telemetry sink error: {}", self.0)
    }
}

impl std::error::Error for ComposedSinkError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&*self.0)
    }
}

/// Multicasts events to two sinks in parallel (operator `+`).
///
/// Both sinks receive all events. If either sink fails, the error is propagated.
///
/// # Example
///
/// ```rust
/// use ninelives::telemetry::{LogSink, MemorySink, MulticastSink};
/// use tower::Service;
///
/// let log = LogSink;
/// let memory = MemorySink::new();
/// let combined = MulticastSink::new(log, memory);
/// // Both sinks will receive all events
/// ```
#[derive(Clone)]
pub struct MulticastSink<A, B> {
    sink_a: A,
    sink_b: B,
}

impl<A, B> MulticastSink<A, B> {
    /// Create a new multicast sink that sends events to both `sink_a` and `sink_b`.
    pub fn new(sink_a: A, sink_b: B) -> Self {
        Self { sink_a, sink_b }
    }
}

impl<A, B> Service<PolicyEvent> for MulticastSink<A, B>
where
    A: tower::Service<PolicyEvent, Response = ()> + Clone + Send + 'static,
    A::Error: std::error::Error + Send + Sync + 'static,
    A::Future: Send + 'static,
    B: tower::Service<PolicyEvent, Response = ()> + Clone + Send + 'static,
    B::Error: std::error::Error + Send + Sync + 'static,
    B::Future: Send + 'static,
{
    type Response = ();
    type Error = ComposedSinkError;
    type Future = Pin<Box<dyn std::future::Future<Output = Result<(), Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, event: PolicyEvent) -> Self::Future {
        let mut sink_a = self.sink_a.clone();
        let mut sink_b = self.sink_b.clone();
        let event_clone = event.clone();

        Box::pin(async move {
            // Call both sinks concurrently
            let (res_a, res_b) = tokio::join!(sink_a.call(event), sink_b.call(event_clone));

            res_a.map_err(|e| ComposedSinkError(Box::new(e)))?;
            res_b.map_err(|e| ComposedSinkError(Box::new(e)))?;

            Ok(())
        })
    }
}

impl<A, B> TelemetrySink for MulticastSink<A, B>
where
    A: tower::Service<PolicyEvent, Response = ()> + Clone + Send + 'static,
    A::Error: std::error::Error + Send + Sync + 'static,
    A::Future: Send + 'static,
    B: tower::Service<PolicyEvent, Response = ()> + Clone + Send + 'static,
    B::Error: std::error::Error + Send + Sync + 'static,
    B::Future: Send + 'static,
{
    type SinkError = ComposedSinkError;
}

/// Sends events to primary sink, falling back to secondary on error (operator `|`).
///
/// # Example
///
/// ```rust
/// use ninelives::telemetry::{LogSink, MemorySink, FallbackSink};
/// use tower::Service;
///
/// let primary = LogSink;
/// let fallback = MemorySink::new();
/// let combined = FallbackSink::new(primary, fallback);
/// // Try primary, use fallback if it fails
/// ```
#[derive(Clone)]
pub struct FallbackSink<A, B> {
    primary: A,
    fallback: B,
}

impl<A, B> FallbackSink<A, B> {
    /// Create a new fallback sink that tries `primary` first, then `fallback` on error.
    pub fn new(primary: A, fallback: B) -> Self {
        Self { primary, fallback }
    }
}

impl<A, B> Service<PolicyEvent> for FallbackSink<A, B>
where
    A: tower::Service<PolicyEvent, Response = ()> + Clone + Send + 'static,
    A::Error: std::error::Error + Send + Sync + 'static,
    A::Future: Send + 'static,
    B: tower::Service<PolicyEvent, Response = ()> + Clone + Send + 'static,
    B::Error: std::error::Error + Send + Sync + 'static,
    B::Future: Send + 'static,
{
    type Response = ();
    type Error = ComposedSinkError;
    type Future = Pin<Box<dyn std::future::Future<Output = Result<(), Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, event: PolicyEvent) -> Self::Future {
        let mut primary = self.primary.clone();
        let mut fallback = self.fallback.clone();
        let event_clone = event.clone();

        Box::pin(async move {
            match primary.call(event).await {
                Ok(()) => Ok(()),
                Err(_) => {
                    // Primary failed, try fallback
                    fallback.call(event_clone).await.map_err(|e| ComposedSinkError(Box::new(e)))
                }
            }
        })
    }
}

impl<A, B> TelemetrySink for FallbackSink<A, B>
where
    A: tower::Service<PolicyEvent, Response = ()> + Clone + Send + 'static,
    A::Error: std::error::Error + Send + Sync + 'static,
    A::Future: Send + 'static,
    B: tower::Service<PolicyEvent, Response = ()> + Clone + Send + 'static,
    B::Error: std::error::Error + Send + Sync + 'static,
    B::Future: Send + 'static,
{
    type SinkError = ComposedSinkError;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::future::Future;

    #[test]
    fn test_retry_event_display() {
        let event = RetryEvent::Attempt { attempt: 2, delay: Duration::from_millis(100) };
        assert!(event.to_string().contains("Attempt"));
        assert!(event.to_string().contains("#2"));
    }

    #[test]
    fn test_circuit_breaker_event_display() {
        let event = CircuitBreakerEvent::Opened { failure_count: 5 };
        assert!(event.to_string().contains("Opened"));
        assert!(event.to_string().contains("5"));
    }

    #[test]
    fn test_bulkhead_event_display() {
        let event = BulkheadEvent::Rejected { active_count: 10, max_concurrency: 10 };
        assert!(event.to_string().contains("Rejected"));
        assert!(event.to_string().contains("10/10"));
    }

    #[test]
    fn test_policy_event_clone() {
        let event = PolicyEvent::Retry(RetryEvent::Attempt {
            attempt: 1,
            delay: Duration::from_millis(50),
        });
        let cloned = event.clone();
        assert_eq!(event, cloned);
    }

    #[tokio::test]
    async fn test_null_sink() {
        use tower::Service;

        let mut sink = NullSink;
        let event = PolicyEvent::Retry(RetryEvent::Attempt {
            attempt: 1,
            delay: Duration::from_millis(100),
        });

        // Should succeed without error
        sink.call(event).await.unwrap();
    }

    #[tokio::test]
    async fn test_memory_sink() {
        use tower::Service;

        let mut sink = MemorySink::with_capacity(2);
        assert!(sink.is_empty());
        assert_eq!(sink.len(), 0);

        let event1 = PolicyEvent::Retry(RetryEvent::Attempt {
            attempt: 1,
            delay: Duration::from_millis(100),
        });
        let event2 = PolicyEvent::CircuitBreaker(CircuitBreakerEvent::Opened { failure_count: 5 });
        let event3 =
            PolicyEvent::Timeout(TimeoutEvent::Occurred { timeout: Duration::from_secs(1) });

        sink.call(event1.clone()).await.unwrap();
        sink.call(event2.clone()).await.unwrap();
        sink.call(event3.clone()).await.unwrap(); // should evict oldest

        assert_eq!(sink.len(), 2);
        assert!(!sink.is_empty());

        assert_eq!(sink.evicted(), 1);

        let events = sink.events();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0], event2);
        assert_eq!(events[1], event3);

        sink.clear();
        assert!(sink.is_empty());
    }

    #[tokio::test]
    async fn test_streaming_sink_drop_counts() {
        use tower::Service;

        let sink = StreamingSink::new(1);
        let mut tx = sink.clone();

        // No subscriber; first send drops
        tx.call(PolicyEvent::Bulkhead(BulkheadEvent::Rejected {
            active_count: 1,
            max_concurrency: 1,
        }))
        .await
        .unwrap();

        assert!(sink.dropped_count() >= 1);
    }

    #[tokio::test]
    async fn test_streaming_sink_last_drop_updates() {
        use tower::Service;

        let sink = StreamingSink::new(1);
        let mut tx = sink.clone();

        // drop once to set last_drop
        tx.call(PolicyEvent::Retry(RetryEvent::Attempt {
            attempt: 1,
            delay: Duration::from_millis(5),
        }))
        .await
        .unwrap();

        assert!(sink.last_drop().is_some());
    }

    #[tokio::test]
    async fn test_streaming_sink_delivers_to_subscriber() {
        use tower::Service;
        let sink = StreamingSink::new(8);
        let mut rx = sink.subscribe();
        let mut tx = sink.clone();

        tx.call(PolicyEvent::Timeout(TimeoutEvent::Occurred { timeout: Duration::from_millis(5) }))
            .await
            .unwrap();
        let got = rx.recv().await.expect("message");
        assert!(matches!(got, PolicyEvent::Timeout(_)));
    }

    #[tokio::test]
    async fn test_emit_best_effort_swallows_errors() {
        #[derive(Clone)]
        struct Fails;
        impl TelemetrySink for Fails {
            type SinkError = std::io::Error;
        }
        impl tower_service::Service<PolicyEvent> for Fails {
            type Response = ();
            type Error = std::io::Error;
            type Future = Pin<Box<dyn Future<Output = Result<(), Self::Error>> + Send>>;
            fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
                Poll::Ready(Ok(()))
            }
            fn call(&mut self, _req: PolicyEvent) -> Self::Future {
                Box::pin(async { Err(std::io::Error::new(std::io::ErrorKind::Other, "fail")) })
            }
        }

        // Should not panic even though sink errors
        emit_best_effort(Fails, PolicyEvent::Timeout(TimeoutEvent::Occurred { timeout: Duration::from_millis(1) }))
            .await;
    }

    #[test]
    fn test_policy_event_request_variants_display() {
        let ok = PolicyEvent::Request(RequestOutcome::Success { duration: Duration::from_millis(5) });
        let err = PolicyEvent::Request(RequestOutcome::Failure { duration: Duration::from_millis(7) });
        assert!(format!("{}", ok).contains("Success"));
        assert!(format!("{}", err).contains("Failure"));
    }

    #[tokio::test]
    async fn test_log_sink() {
        use tower::Service;

        let mut sink = LogSink;
        let event =
            PolicyEvent::Timeout(TimeoutEvent::Occurred { timeout: Duration::from_secs(1) });

        // Should succeed without error
        sink.call(event).await.unwrap();
    }
}
