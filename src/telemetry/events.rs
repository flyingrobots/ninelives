use std::time::Duration;
use std::fmt;

#[cfg(feature = "telemetry-json")]
use serde_json::json;

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

#[cfg_attr(not(feature = "telemetry-json"), allow(dead_code))]
#[inline]
fn clamp_u64(val: u128) -> u64 {
    val.min(u128::from(u64::MAX)) as u64
}

/// Convert a PolicyEvent into a JSON value for sinks.
#[cfg(feature = "telemetry-json")]
pub fn event_to_json(event: &PolicyEvent) -> serde_json::Value {
    match event {
        PolicyEvent::Retry(r) => match r {
            RetryEvent::Attempt { attempt, delay } => json!({
                "kind": "retry_attempt",
                "attempt": *attempt,
                "delay_ms": clamp_u64(delay.as_millis()),
            }),
            RetryEvent::Exhausted { total_attempts, total_duration } => json!({
                "kind": "retry_exhausted",
                "attempts": *total_attempts,
                "duration_ms": clamp_u64(total_duration.as_millis()),
            }),
        },
        PolicyEvent::CircuitBreaker(c) => match c {
            CircuitBreakerEvent::Opened { failure_count } => {
                json!({ "kind": "circuit_opened", "failures": *failure_count })
            }
            CircuitBreakerEvent::HalfOpen => json!({ "kind": "circuit_half_open" }),
            CircuitBreakerEvent::Closed => json!({ "kind": "circuit_closed" }),
        },
        PolicyEvent::Bulkhead(b) => match b {
            BulkheadEvent::Acquired { active_count, max_concurrency } => json!({
                "kind": "bulkhead_acquired",
                "active": *active_count,
                "max": *max_concurrency
            }),
            BulkheadEvent::Rejected { active_count, max_concurrency } => json!({
                "kind": "bulkhead_rejected",
                "active": *active_count,
                "max": *max_concurrency
            }),
        },
        PolicyEvent::Timeout(t) => match t {
            TimeoutEvent::Occurred { timeout } => json!({
                "kind": "timeout",
                "timeout_ms": clamp_u64(timeout.as_millis())
            }),
        },
        PolicyEvent::Request(r) => match r {
            RequestOutcome::Success { duration } => json!({
                "kind": "request_success",
                "duration_ms": clamp_u64(duration.as_millis())
            }),
            RequestOutcome::Failure { duration } => json!({
                "kind": "request_failure",
                "duration_ms": clamp_u64(duration.as_millis())
            }),
        },
    }
}

#[cfg(all(test, feature = "telemetry-json"))]
mod json_tests {
    use super::*;

    #[test]
    fn retry_attempt_json() {
        let v = event_to_json(&PolicyEvent::Retry(RetryEvent::Attempt {
            attempt: 3,
            delay: Duration::from_millis(150),
        }));
        assert_eq!(v["kind"], "retry_attempt");
        assert_eq!(v["attempt"], 3);
        assert_eq!(v["delay_ms"], 150);
    }

    #[test]
    fn retry_exhausted_json() {
        let v = event_to_json(&PolicyEvent::Retry(RetryEvent::Exhausted {
            total_attempts: 5,
            total_duration: Duration::from_millis(1200),
        }));
        assert_eq!(v["kind"], "retry_exhausted");
        assert_eq!(v["attempts"], 5);
        assert_eq!(v["duration_ms"], 1200);
    }

    #[test]
    fn circuit_opened_json() {
        let v = event_to_json(&PolicyEvent::CircuitBreaker(CircuitBreakerEvent::Opened {
            failure_count: 4,
        }));
        assert_eq!(v["kind"], "circuit_opened");
        assert_eq!(v["failures"], 4);
    }

    #[test]
    fn circuit_half_open_json() {
        let v = event_to_json(&PolicyEvent::CircuitBreaker(CircuitBreakerEvent::HalfOpen));
        assert_eq!(v["kind"], "circuit_half_open");
    }

    #[test]
    fn circuit_closed_json() {
        let v = event_to_json(&PolicyEvent::CircuitBreaker(CircuitBreakerEvent::Closed));
        assert_eq!(v["kind"], "circuit_closed");
    }

    #[test]
    fn bulkhead_acquired_json() {
        let v = event_to_json(&PolicyEvent::Bulkhead(BulkheadEvent::Acquired {
            active_count: 2,
            max_concurrency: 5,
        }));
        assert_eq!(v["kind"], "bulkhead_acquired");
        assert_eq!(v["active"], 2);
        assert_eq!(v["max"], 5);
    }

    #[test]
    fn bulkhead_rejected_json() {
        let v = event_to_json(&PolicyEvent::Bulkhead(BulkheadEvent::Rejected {
            active_count: 5,
            max_concurrency: 5,
        }));
        assert_eq!(v["kind"], "bulkhead_rejected");
        assert_eq!(v["active"], 5);
        assert_eq!(v["max"], 5);
    }

    #[test]
    fn timeout_json() {
        let v = event_to_json(&PolicyEvent::Timeout(TimeoutEvent::Occurred {
            timeout: Duration::from_millis(2500),
        }));
        assert_eq!(v["kind"], "timeout");
        assert_eq!(v["timeout_ms"], 2500);
    }

    #[test]
    fn request_success_json() {
        let v = event_to_json(&PolicyEvent::Request(RequestOutcome::Success {
            duration: Duration::from_millis(42),
        }));
        assert_eq!(v["kind"], "request_success");
        assert_eq!(v["duration_ms"], 42);
    }

    #[test]
    fn request_failure_json() {
        let v = event_to_json(&PolicyEvent::Request(RequestOutcome::Failure {
            duration: Duration::from_millis(99),
        }));
        assert_eq!(v["kind"], "request_failure");
        assert_eq!(v["duration_ms"], 99);
    }

    #[test]
    fn telemetry_json_contains_no_auth_fields() {
        let v = event_to_json(&PolicyEvent::Request(RequestOutcome::Success {
            duration: Duration::from_millis(1),
        }));
        let s = serde_json::to_string(&v).unwrap();
        assert!(
            !s.contains("auth"),
            "telemetry JSON should not carry auth payloads; got {s}"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn test_policy_event_request_variants_display() {
        let ok =
            PolicyEvent::Request(RequestOutcome::Success { duration: Duration::from_millis(5) });
        let err =
            PolicyEvent::Request(RequestOutcome::Failure { duration: Duration::from_millis(7) });
        assert!(format!("{}", ok).contains("Success"));
        assert!(format!("{}", err).contains("Failure"));
    }
}
