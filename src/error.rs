//! Error types for resilience policies
use std::fmt;
use std::sync::Arc;
use std::time::Duration;
/// Cap the number of stored failures inside RetryExhausted to avoid unbounded growth.
pub const MAX_RETRY_FAILURES: usize = 10;
/// Unified error type for all resilience policies
#[derive(Debug, Clone)]
pub enum ResilienceError<E> {
    /// The operation exceeded the timeout duration
    Timeout { elapsed: Duration, timeout: Duration },
    /// The bulkhead rejected the operation due to capacity
    Bulkhead { in_flight: usize, max: usize },
    /// The circuit breaker is open
    CircuitOpen { failure_count: usize, open_duration: Duration },
    /// All retry attempts were exhausted
    RetryExhausted { attempts: usize, failures: Arc<Vec<E>> },
    /// The underlying operation failed
    Inner(E),
}
impl<E: fmt::Display> fmt::Display for ResilienceError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Timeout { elapsed, timeout } => {
                write!(f, "operation timed out after {:?} (limit: {:?})", elapsed, timeout)
            }
            Self::Bulkhead { in_flight, max } => {
                write!(f, "bulkhead rejected request ({} in-flight, max {})", in_flight, max)
            }
            Self::CircuitOpen { failure_count, open_duration } => {
                write!(
                    f,
                    "circuit breaker open ({} failures, open for {:?})",
                    failure_count, open_duration
                )
            }
            Self::RetryExhausted { attempts, failures } => {
                let recorded = failures.len();
                let truncated_note = if recorded < *attempts {
                    format!(" (recorded last {} failures)", recorded)
                } else {
                    String::new()
                };
                if let Some(last) = failures.last() {
                    write!(
                        f,
                        "retry exhausted after {} attempts{}; last error: {}",
                        attempts, truncated_note, last
                    )
                } else {
                    write!(
                        f,
                        "retry exhausted after {} attempts{}; no recorded failures",
                        attempts, truncated_note
                    )
                }
            }
            Self::Inner(e) => write!(f, "{}", e),
        }
    }
}
impl<E: std::error::Error + 'static> std::error::Error for ResilienceError<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Inner(e) => Some(e),
            Self::RetryExhausted { failures, .. } => {
                failures.last().map(|e| e as &dyn std::error::Error)
            }
            _ => None,
        }
    }
}
impl<E> ResilienceError<E> {
    /// Construct a `RetryExhausted` variant while enforcing the `MAX_RETRY_FAILURES` cap by keeping the most recent failures.
    pub fn retry_exhausted(attempts: usize, failures: Vec<E>) -> Self {
        let trimmed = if failures.len() > MAX_RETRY_FAILURES {
            failures.into_iter().rev().take(MAX_RETRY_FAILURES).rev().collect()
        } else {
            failures
        };
        ResilienceError::RetryExhausted { attempts, failures: Arc::new(trimmed) }
    }
    /// Check if this error is due to timeout
    pub fn is_timeout(&self) -> bool {
        matches!(self, Self::Timeout { .. })
    }
    /// Check if this error is due to circuit breaker
    pub fn is_circuit_open(&self) -> bool {
        matches!(self, Self::CircuitOpen { .. })
    }
    /// Check if this error is due to bulkhead rejection
    pub fn is_bulkhead(&self) -> bool {
        matches!(self, Self::Bulkhead { .. })
    }
    /// Check if this error is due to retry exhaustion
    pub fn is_retry_exhausted(&self) -> bool {
        matches!(self, Self::RetryExhausted { .. })
    }
    /// Get the inner error if this is an Inner variant
    pub fn into_inner(self) -> Option<E> {
        match self {
            Self::Inner(e) => Some(e),
            _ => None,
        }
    }
    /// Access all recorded failures for RetryExhausted, if present.
    pub fn failures(&self) -> Option<&[E]> {
        match self {
            Self::RetryExhausted { failures, .. } => Some(failures.as_slice()),
            _ => None,
        }
    }
    /// Check if this error wraps an inner error.
    pub fn is_inner(&self) -> bool {
        matches!(self, Self::Inner(_))
    }
    /// Borrow the inner error if present.
    pub fn as_inner(&self) -> Option<&E> {
        match self {
            Self::Inner(e) => Some(e),
            _ => None,
        }
    }
    /// Mutably borrow the inner error if present.
    pub fn as_inner_mut(&mut self) -> Option<&mut E> {
        match self {
            Self::Inner(e) => Some(e),
            _ => None,
        }
    }
    /// Access timeout details if this is a timeout error.
    pub fn timeout_details(&self) -> Option<(Duration, Duration)> {
        match self {
            Self::Timeout { elapsed, timeout } => Some((*elapsed, *timeout)),
            _ => None,
        }
    }
    /// Access circuit-open remaining duration if present.
    pub fn circuit_open_duration(&self) -> Option<Duration> {
        match self {
            Self::CircuitOpen { open_duration, .. } => Some(*open_duration),
            _ => None,
        }
    }
    /// Access bulkhead capacity info as (in_flight, max).
    pub fn bulkhead_capacity(&self) -> Option<(usize, usize)> {
        match self {
            Self::Bulkhead { in_flight, max } => Some((*in_flight, *max)),
            _ => None,
        }
    }
    /// Access retry exhaustion info as (attempts, recorded_failures).
    pub fn retry_exhausted_info(&self) -> Option<(usize, usize)> {
        match self {
            Self::RetryExhausted { attempts, failures } => Some((*attempts, failures.len())),
            _ => None,
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;
    use std::fmt;
    use std::io;
    #[derive(Debug, Clone, PartialEq, Eq)]
    struct DummyError(&'static str);
    impl fmt::Display for DummyError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "{}", self.0)
        }
    }
    impl std::error::Error for DummyError {}
    #[test]
    fn timeout_error_display() {
        let err: ResilienceError<io::Error> = ResilienceError::Timeout {
            elapsed: Duration::from_millis(5100),
            timeout: Duration::from_secs(5),
        };
        let msg = format!("{}", err);
        assert!(msg.contains("timed out"));
        assert!(msg.contains("5.1"));
    }
    #[test]
    fn bulkhead_error_display() {
        let err: ResilienceError<io::Error> = ResilienceError::Bulkhead { in_flight: 50, max: 50 };
        let msg = format!("{}", err);
        assert!(msg.contains("bulkhead"));
        assert!(msg.contains("50"));
    }
    #[test]
    fn circuit_open_error_display() {
        let err: ResilienceError<io::Error> = ResilienceError::CircuitOpen {
            failure_count: 10,
            open_duration: Duration::from_secs(30),
        };
        let msg = format!("{}", err);
        assert!(msg.contains("circuit breaker"));
        assert!(msg.contains("10"));
    }
    #[test]
    fn retry_exhausted_display_includes_last_error() {
        let err: ResilienceError<DummyError> = ResilienceError::RetryExhausted {
            attempts: 3,
            failures: Arc::new(vec![DummyError("first"), DummyError("last")]),
        };
        let msg = format!("{}", err);
        assert!(msg.contains("3"));
        assert!(msg.contains("last error"));
        assert!(msg.contains("last"));
    }
    #[test]
    fn retry_exhausted_display_handles_empty_failures() {
        let err: ResilienceError<DummyError> = ResilienceError::retry_exhausted(3, vec![]);
        let msg = format!("{}", err);
        assert!(msg.contains("3"));
        assert!(msg.contains("no recorded failures"));
        assert!(!msg.ends_with(": "));
    }
    #[test]
    fn is_timeout_check() {
        let err: ResilienceError<io::Error> = ResilienceError::Timeout {
            elapsed: Duration::from_secs(1),
            timeout: Duration::from_secs(1),
        };
        assert!(err.is_timeout());
        assert!(!err.is_circuit_open());
        assert!(!err.is_bulkhead());
    }
    #[test]
    fn into_inner_extracts_error() {
        let io_err = io::Error::new(io::ErrorKind::Other, "test");
        let err = ResilienceError::Inner(io_err);
        let extracted = err.into_inner().unwrap();
        assert_eq!(extracted.to_string(), "test");
    }
    #[test]
    fn accessor_methods_return_expected_data() {
        let timeout = ResilienceError::<DummyError>::Timeout {
            elapsed: Duration::from_millis(10),
            timeout: Duration::from_millis(20),
        };
        assert_eq!(
            timeout.timeout_details(),
            Some((Duration::from_millis(10), Duration::from_millis(20)))
        );
        assert!(timeout.bulkhead_capacity().is_none());
        let bulk = ResilienceError::<DummyError>::Bulkhead { in_flight: 2, max: 5 };
        assert_eq!(bulk.bulkhead_capacity(), Some((2, 5)));
        assert!(bulk.timeout_details().is_none());
        let circuit = ResilienceError::<DummyError>::CircuitOpen {
            failure_count: 3,
            open_duration: Duration::from_millis(50),
        };
        assert_eq!(circuit.circuit_open_duration(), Some(Duration::from_millis(50)));
        let failures = vec![DummyError("one"), DummyError("two")];
        let retry = ResilienceError::retry_exhausted(5, failures.clone());
        assert_eq!(retry.retry_exhausted_info(), Some((5, failures.len())));
        assert_eq!(retry.failures().unwrap(), failures.as_slice());
        let inner = ResilienceError::Inner(DummyError("x"));
        assert!(inner.failures().is_none());
        assert!(inner.retry_exhausted_info().is_none());
    }
    #[test]
    fn source_is_none_for_timeout() {
        let err: ResilienceError<DummyError> = ResilienceError::Timeout {
            elapsed: Duration::from_secs(1),
            timeout: Duration::from_secs(2),
        };
        assert!(err.source().is_none());
    }
    #[test]
    fn predicates_cover_all_variants() {
        let timeout: ResilienceError<DummyError> = ResilienceError::Timeout {
            elapsed: Duration::from_secs(1),
            timeout: Duration::from_secs(2),
        };
        assert!(timeout.is_timeout());
        assert!(!timeout.is_circuit_open());
        let bulkhead: ResilienceError<DummyError> =
            ResilienceError::Bulkhead { in_flight: 1, max: 1 };
        assert!(bulkhead.is_bulkhead());
        let circuit: ResilienceError<DummyError> = ResilienceError::CircuitOpen {
            failure_count: 1,
            open_duration: Duration::from_secs(1),
        };
        assert!(circuit.is_circuit_open());
        let retry: ResilienceError<DummyError> =
            ResilienceError::RetryExhausted { attempts: 2, failures: Arc::new(vec![]) };
        assert!(retry.is_retry_exhausted());
    }
    #[test]
    fn as_inner_accessors_work() {
        let mut err: ResilienceError<DummyError> = ResilienceError::Inner(DummyError("x"));
        assert!(err.is_inner());
        assert_eq!(err.as_inner().unwrap().0, "x");
        if let Some(inner) = err.as_inner_mut() {
            inner.0 = "y";
        }
        assert_eq!(err.as_inner().unwrap().0, "y");
    }
}
