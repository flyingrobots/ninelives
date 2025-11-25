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
use std::time::Duration;

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
            RetryEvent::Exhausted {
                total_attempts,
                total_duration,
            } => write!(
                f,
                "Exhausted(attempts={}, duration={:?})",
                total_attempts, total_duration
            ),
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
            BulkheadEvent::Acquired {
                active_count,
                max_concurrency,
            } => write!(f, "Acquired({}/{})", active_count, max_concurrency),
            BulkheadEvent::Rejected {
                active_count,
                max_concurrency,
            } => write!(f, "Rejected({}/{})", active_count, max_concurrency),
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
}

impl MemorySink {
    /// Creates a new empty memory sink.
    pub fn new() -> Self {
        Self {
            events: Arc::new(Mutex::new(Vec::new())),
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
        self.events.lock().unwrap().push(event);
        Box::pin(async { Ok(()) })
    }
}

impl TelemetrySink for MemorySink {
    type SinkError = Infallible;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retry_event_display() {
        let event = RetryEvent::Attempt {
            attempt: 2,
            delay: Duration::from_millis(100),
        };
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
        let event = BulkheadEvent::Rejected {
            active_count: 10,
            max_concurrency: 10,
        };
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

        let mut sink = MemorySink::new();
        assert!(sink.is_empty());
        assert_eq!(sink.len(), 0);

        let event1 = PolicyEvent::Retry(RetryEvent::Attempt {
            attempt: 1,
            delay: Duration::from_millis(100),
        });
        let event2 = PolicyEvent::CircuitBreaker(CircuitBreakerEvent::Opened {
            failure_count: 5,
        });

        sink.call(event1.clone()).await.unwrap();
        sink.call(event2.clone()).await.unwrap();

        assert_eq!(sink.len(), 2);
        assert!(!sink.is_empty());

        let events = sink.events();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0], event1);
        assert_eq!(events[1], event2);

        sink.clear();
        assert!(sink.is_empty());
    }

    #[tokio::test]
    async fn test_log_sink() {
        use tower::Service;

        let mut sink = LogSink;
        let event = PolicyEvent::Timeout(TimeoutEvent::Occurred {
            timeout: Duration::from_secs(1),
        });

        // Should succeed without error
        sink.call(event).await.unwrap();
    }
}
