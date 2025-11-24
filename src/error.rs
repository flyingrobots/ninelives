//! Error types for resilience policies

use std::fmt;
use std::time::Duration;

/// Cap the number of stored failures inside RetryExhausted to avoid unbounded growth.
pub const MAX_RETRY_FAILURES: usize = 10;

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

impl<E: Clone> Clone for ResilienceError<E> {
    fn clone(&self) -> Self {
        match self {
            Self::Timeout { elapsed, timeout } => {
                Self::Timeout { elapsed: *elapsed, timeout: *timeout }
            }
            Self::Bulkhead { in_flight, max } => {
                Self::Bulkhead { in_flight: *in_flight, max: *max }
            }
            Self::CircuitOpen { failure_count, open_duration } => {
                Self::CircuitOpen { failure_count: *failure_count, open_duration: *open_duration }
            }
            Self::RetryExhausted { attempts, failures } => {
                Self::RetryExhausted { attempts: *attempts, failures: failures.clone() }
            }
            Self::Inner(e) => Self::Inner(e.clone()),
        }
    }
}

impl<E: PartialEq> PartialEq for ResilienceError<E> {
    fn eq(&self, other: &Self) -> bool {
        use ResilienceError::*;
        match (self, other) {
            (Timeout { elapsed: a1, timeout: b1 }, Timeout { elapsed: a2, timeout: b2 }) => {
                a1 == a2 && b1 == b2
            }
            (Bulkhead { in_flight: a1, max: b1 }, Bulkhead { in_flight: a2, max: b2 }) => {
                a1 == a2 && b1 == b2
            }
            (
                CircuitOpen { failure_count: f1, open_duration: d1 },
                CircuitOpen { failure_count: f2, open_duration: d2 },
            ) => f1 == f2 && d1 == d2,
            (
                RetryExhausted { attempts: a1, failures: f1 },
                RetryExhausted { attempts: a2, failures: f2 },
            ) => a1 == a2 && f1 == f2,
            (Inner(e1), Inner(e2)) => e1 == e2,
            _ => false,
        }
    }
}

impl<E: Eq> Eq for ResilienceError<E> {}

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
                let last = failures.last().map(|e| e.to_string()).unwrap_or_default();
                write!(
                    f,
                    "retry exhausted after {} attempts ({} failures), last error: {}",
                    attempts,
                    failures.len(),
                    last
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
            failures: vec![DummyError("first"), DummyError("last")],
        };
        let msg = format!("{}", err);
        assert!(msg.contains("3"));
        assert!(msg.contains("last error"));
        assert!(msg.contains("last"));
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
    fn source_returns_last_failure_for_retry_exhausted() {
        let err: ResilienceError<DummyError> = ResilienceError::RetryExhausted {
            attempts: 3,
            failures: vec![DummyError("a"), DummyError("b")],
        };
        let src = err.source().unwrap();
        assert_eq!(src.to_string(), "b");
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
            ResilienceError::RetryExhausted { attempts: 2, failures: vec![] };
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
