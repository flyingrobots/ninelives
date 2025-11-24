//! Error types for resilience policies

use std::fmt;
use std::time::Duration;

/// Unified error type for all resilience policies
#[derive(Debug)]
pub enum ResilienceError<E> {
    /// The operation exceeded the timeout duration
    Timeout { elapsed: Duration, timeout: Duration },
    /// The bulkhead rejected the operation due to capacity
    Bulkhead { in_flight: usize, max: usize },
    /// The circuit breaker is open
    CircuitOpen { failure_count: usize, open_duration: Duration },
    /// All retry attempts were exhausted
    RetryExhausted { attempts: usize, failures: Vec<E> },
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
                write!(
                    f,
                    "retry exhausted after {} attempts ({} failures)",
                    attempts,
                    failures.len()
                )
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

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
}
