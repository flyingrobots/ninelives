//! Backoff strategies for retry policies.
//!
//! Provides constant, linear, and exponential strategies with optional caps. Attempt semantics:
//! attempt index `0` represents the initial call (no delay), and retries start at `attempt = 1`;
//! all strategies must honor this 0-based contract consistently.
//! Delays saturate at a documented maximum to avoid overflow.
//! Invariants:
//! - attempt `0` always returns `0` delay.
//! - delays are non-decreasing as attempts increase.
//! - `with_max` caps delays without exceeding either the provided cap or `MAX_BACKOFF`.
//! - strategies never panic on large attempts; they saturate instead.
//!
//! Example
//! ```rust
//! use std::time::Duration;
//! use ninelives::Backoff;
//!
//! let backoff = Backoff::exponential(Duration::from_millis(100))
//!     .with_max(Duration::from_secs(2))
//!     .unwrap();
//! assert_eq!(backoff.delay(0), Duration::from_millis(0)); // initial call
//! assert_eq!(backoff.delay(1), Duration::from_millis(100));
//! assert_eq!(backoff.delay(2), Duration::from_millis(200));
//! assert_eq!(backoff.delay(6), Duration::from_secs(2)); // capped
//! ```
//!
//! Overflow behavior: computations that would overflow saturate to `MAX_BACKOFF` (1 day). Attempts
//! greater than `u32::MAX` are clamped when computing multipliers.

use std::fmt;
use std::sync::Arc;
use std::time::Duration;

/// Maximum delay used when calculations overflow (1 day).
pub const MAX_BACKOFF: Duration = Duration::from_secs(24 * 60 * 60);

/// Errors returned by backoff configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackoffError {
    /// Provided maximum delay was zero.
    MaxMustBePositive,
    /// Maximum delay was smaller than the base delay.
    MaxLessThanBase {
        /// Base delay requested.
        base: Duration,
        /// Maximum delay provided (must be >= base).
        max: Duration,
    },
}

impl fmt::Display for BackoffError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BackoffError::MaxMustBePositive => write!(f, "max must be greater than zero"),
            BackoffError::MaxLessThanBase { base, max } => {
                write!(f, "max ({:?}) must be >= base ({:?})", max, base)
            }
        }
    }
}

impl std::error::Error for BackoffError {}

impl BackoffError {
    /// Stable identifier for logging/metrics.
    pub fn code(&self) -> &'static str {
        match self {
            BackoffError::MaxMustBePositive => "MaxMustBePositive",
            BackoffError::MaxLessThanBase { .. } => "MaxLessThanBase",
        }
    }

    /// Convenience accessor equivalent to `to_string()`.
    pub fn message(&self) -> String {
        self.to_string()
    }
}

/// Trait implemented by all backoff strategies.
pub trait BackoffStrategy: Send + Sync + fmt::Debug {
    /// Compute the delay for the given attempt (0-based; 0 = no delay).
    fn delay(&self, attempt: usize) -> Duration;
}

/// Constant backoff strategy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConstantBackoff {
    delay: Duration,
}

impl ConstantBackoff {
    /// Create a constant backoff with the provided delay.
    pub fn new(delay: Duration) -> Self {
        Self { delay }
    }

    /// Compute the delay for an attempt (0 => zero).
    pub fn delay(&self, attempt: usize) -> Duration {
        BackoffStrategy::delay(self, attempt)
    }
}

impl BackoffStrategy for ConstantBackoff {
    fn delay(&self, attempt: usize) -> Duration {
        if attempt == 0 {
            Duration::from_millis(0)
        } else {
            self.delay
        }
    }
}

/// Linear backoff strategy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinearBackoff {
    base: Duration,
    max: Option<Duration>,
}

impl LinearBackoff {
    /// Create a linear backoff with the given base delay.
    pub fn new(base: Duration) -> Self {
        Self { base, max: None }
    }

    /// Cap the delay at `max` (must be >= base and non-zero).
    pub fn with_max(mut self, max: Duration) -> Result<Self, BackoffError> {
        if max.is_zero() {
            return Err(BackoffError::MaxMustBePositive);
        }
        if max < self.base {
            return Err(BackoffError::MaxLessThanBase { base: self.base, max });
        }
        self.max = Some(max);
        Ok(self)
    }

    /// Compute the delay for an attempt (0 => zero).
    pub fn delay(&self, attempt: usize) -> Duration {
        BackoffStrategy::delay(self, attempt)
    }
}

impl BackoffStrategy for LinearBackoff {
    fn delay(&self, attempt: usize) -> Duration {
        if attempt == 0 {
            return Duration::from_millis(0);
        }
        let attempt_u32 = attempt.min(u32::MAX as usize) as u32; // clamp to prevent truncation/overflow
        let linear = self.base.checked_mul(attempt_u32).unwrap_or(MAX_BACKOFF);
        let capped = self.max.map(|m| linear.min(m)).unwrap_or(linear);
        capped.min(MAX_BACKOFF)
    }
}

/// Exponential backoff strategy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExponentialBackoff {
    base: Duration,
    max: Option<Duration>,
}

impl ExponentialBackoff {
    /// Create an exponential backoff with the given base delay.
    pub fn new(base: Duration) -> Self {
        Self { base, max: None }
    }

    /// Cap the delay at `max` (must be >= base and non-zero).
    pub fn with_max(mut self, max: Duration) -> Result<Self, BackoffError> {
        if max.is_zero() {
            return Err(BackoffError::MaxMustBePositive);
        }
        if max < self.base {
            return Err(BackoffError::MaxLessThanBase { base: self.base, max });
        }
        self.max = Some(max);
        Ok(self)
    }

    /// Compute the delay for an attempt (0 => zero).
    pub fn delay(&self, attempt: usize) -> Duration {
        BackoffStrategy::delay(self, attempt)
    }
}

impl BackoffStrategy for ExponentialBackoff {
    fn delay(&self, attempt: usize) -> Duration {
        if attempt == 0 {
            return Duration::from_millis(0);
        }
        let exponent = attempt.saturating_sub(1).min(u32::MAX as usize) as u32;
        let multiplier = 2u128.saturating_pow(exponent);
        let base_nanos = self.base.as_nanos().saturating_mul(multiplier);
        let exp_delay = Duration::from_nanos(base_nanos.min(MAX_BACKOFF.as_nanos()) as u64);
        let capped = self.max.map(|m| exp_delay.min(m)).unwrap_or(exp_delay);
        capped.min(MAX_BACKOFF)
    }
}

/// Backoff strategy wrapper preserving the existing API while delegating to concrete strategies.
#[derive(Clone)]
pub struct Backoff {
    strategy: Arc<dyn BackoffStrategy>,
}

impl PartialEq for Backoff {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.strategy, &other.strategy)
    }
}

impl Eq for Backoff {}

impl fmt::Debug for Backoff {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Backoff").finish_non_exhaustive()
    }
}

impl Backoff {
    /// Wrap any strategy that implements `BackoffStrategy`.
    pub fn new<S>(strategy: S) -> Self
    where
        S: BackoffStrategy + 'static,
    {
        Self { strategy: Arc::new(strategy) }
    }

    /// Convenience constructor for a constant backoff strategy.
    pub fn constant(delay: Duration) -> ConstantBackoff {
        ConstantBackoff::new(delay)
    }

    /// Convenience constructor for a linear backoff strategy.
    pub fn linear(base: Duration) -> LinearBackoff {
        LinearBackoff::new(base)
    }

    /// Convenience constructor for an exponential backoff strategy.
    pub fn exponential(base: Duration) -> ExponentialBackoff {
        ExponentialBackoff::new(base)
    }

    /// Calculate the delay for a given attempt number (0-based; 0 = initial call, no delay).
    pub fn delay(&self, attempt: usize) -> Duration {
        self.strategy.delay(attempt)
    }
}

impl BackoffStrategy for Backoff {
    fn delay(&self, attempt: usize) -> Duration {
        self.strategy.delay(attempt)
    }
}

impl From<ConstantBackoff> for Backoff {
    fn from(strategy: ConstantBackoff) -> Self {
        Backoff::new(strategy)
    }
}

impl From<LinearBackoff> for Backoff {
    fn from(strategy: LinearBackoff) -> Self {
        Backoff::new(strategy)
    }
}

impl From<ExponentialBackoff> for Backoff {
    fn from(strategy: ExponentialBackoff) -> Self {
        Backoff::new(strategy)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn with_max_zero_rejected() {
        let err = Backoff::linear(Duration::from_millis(10)).with_max(Duration::ZERO).unwrap_err();
        assert_eq!(err.to_string(), "max must be greater than zero");
    }

    #[test]
    fn max_equal_base_is_allowed_and_caps() {
        let backoff =
            Backoff::linear(Duration::from_millis(10)).with_max(Duration::from_millis(10)).unwrap();
        assert_eq!(backoff.delay(1), Duration::from_millis(10));
        assert_eq!(backoff.delay(5), Duration::from_millis(10));
    }

    #[test]
    fn backoff_error_display_is_human_readable() {
        let err = BackoffError::MaxLessThanBase {
            base: Duration::from_secs(2),
            max: Duration::from_secs(1),
        };
        assert!(
            err.to_string().contains("max") && err.to_string().contains("base"),
            "display should include context"
        );
        assert_eq!(err.code(), "MaxLessThanBase");
        assert!(err.message().contains("max"));
    }

    #[test]
    fn constant_backoff_returns_same_delay() {
        let backoff = Backoff::constant(Duration::from_secs(1));
        assert_eq!(backoff.delay(0), Duration::from_millis(0));
        assert_eq!(backoff.delay(1), Duration::from_secs(1));
        assert_eq!(backoff.delay(2), Duration::from_secs(1));
        assert_eq!(backoff.delay(100), Duration::from_secs(1));
    }

    #[test]
    fn linear_backoff_increases_linearly() {
        let backoff = Backoff::linear(Duration::from_millis(100));
        assert_eq!(backoff.delay(0), Duration::from_millis(0));
        assert_eq!(backoff.delay(1), Duration::from_millis(100));
        assert_eq!(backoff.delay(2), Duration::from_millis(200));
        assert_eq!(backoff.delay(3), Duration::from_millis(300));
        assert_eq!(backoff.delay(10), Duration::from_millis(1000));
    }

    #[test]
    fn delay_handles_zero_attempt() {
        let constant = Backoff::constant(Duration::from_millis(50));
        assert_eq!(constant.delay(0), Duration::from_millis(0));

        let linear = Backoff::linear(Duration::from_millis(50));
        assert_eq!(linear.delay(0), Duration::from_millis(0));

        let exponential = Backoff::exponential(Duration::from_millis(50));
        assert_eq!(exponential.delay(0), Duration::from_millis(0));
    }

    #[test]
    fn exponential_backoff_doubles_each_time() {
        let backoff = Backoff::exponential(Duration::from_millis(100));
        assert_eq!(backoff.delay(1), Duration::from_millis(100)); // 100 * 2^0
        assert_eq!(backoff.delay(2), Duration::from_millis(200)); // 100 * 2^1
        assert_eq!(backoff.delay(3), Duration::from_millis(400)); // 100 * 2^2
        assert_eq!(backoff.delay(4), Duration::from_millis(800)); // 100 * 2^3
        assert_eq!(backoff.delay(5), Duration::from_millis(1600)); // 100 * 2^4
    }

    #[test]
    fn exponential_backoff_respects_max() {
        let backoff = Backoff::exponential(Duration::from_millis(100))
            .with_max(Duration::from_secs(1))
            .unwrap();

        assert_eq!(backoff.delay(1), Duration::from_millis(100));
        assert_eq!(backoff.delay(2), Duration::from_millis(200));
        assert_eq!(backoff.delay(3), Duration::from_millis(400));
        assert_eq!(backoff.delay(4), Duration::from_millis(800));
        assert_eq!(backoff.delay(5), Duration::from_secs(1)); // Capped
        assert_eq!(backoff.delay(10), Duration::from_secs(1)); // Still capped
    }

    #[test]
    fn exponential_backoff_handles_overflow() {
        let backoff = Backoff::exponential(Duration::from_secs(1));
        // Very large attempt should saturate safely
        let huge_attempt: usize = 1_000_000_000;
        let delay = backoff.delay(huge_attempt);
        assert_eq!(delay, MAX_BACKOFF); // Saturated
    }

    #[test]
    fn delays_monotonic_and_capped() {
        let backoff = Backoff::linear(Duration::from_millis(30))
            .with_max(Duration::from_millis(100))
            .unwrap();
        let mut last = Duration::from_millis(0);
        for attempt in 0..20 {
            let d = backoff.delay(attempt);
            assert!(d >= last, "delay regressed at attempt {}", attempt);
            assert!(d <= Duration::from_millis(100));
            last = d;
        }
    }

    #[test]
    fn linear_backoff_handles_overflow() {
        let backoff = Backoff::linear(Duration::from_secs(u64::MAX / 2));
        // Should saturate to max duration instead of panicking
        let huge_attempt: usize = 1_000_000_000;
        let delay = backoff.delay(huge_attempt);
        assert_eq!(delay, MAX_BACKOFF);
    }

    #[test]
    fn with_max_respected_by_linear() {
        let linear =
            Backoff::linear(Duration::from_secs(5)).with_max(Duration::from_secs(7)).unwrap();
        // Linear should respect max
        assert_eq!(linear.delay(2), Duration::from_secs(7));
    }

    #[test]
    fn linear_with_cap_progression() {
        let backoff =
            Backoff::linear(Duration::from_secs(10)).with_max(Duration::from_secs(25)).unwrap();
        assert_eq!(backoff.delay(1), Duration::from_secs(10));
        assert_eq!(backoff.delay(2), Duration::from_secs(20));
        assert_eq!(backoff.delay(3), Duration::from_secs(25)); // capped
        assert_eq!(backoff.delay(10), Duration::from_secs(25)); // still capped
    }

    #[test]
    fn base_greater_than_max_is_rejected() {
        let err = Backoff::linear(Duration::from_secs(100))
            .with_max(Duration::from_secs(50))
            .unwrap_err();
        assert!(matches!(err, BackoffError::MaxLessThanBase { .. }));
    }

    #[test]
    fn zero_base_behaves() {
        let linear = Backoff::linear(Duration::ZERO);
        assert_eq!(linear.delay(5), Duration::ZERO);
        let exp = Backoff::exponential(Duration::ZERO);
        assert_eq!(exp.delay(3), Duration::ZERO);
    }

    #[test]
    fn very_large_attempt_clamps() {
        let backoff = Backoff::exponential(Duration::from_secs(2));
        let delay = backoff.delay((u32::MAX as usize) + 10_000);
        assert_eq!(delay, MAX_BACKOFF);
    }

    #[test]
    fn strategies_convert_into_wrapper() {
        let backoff: Backoff = Backoff::linear(Duration::from_millis(10))
            .with_max(Duration::from_millis(15))
            .unwrap()
            .into();
        assert_eq!(backoff.delay(2), Duration::from_millis(15));
    }
}
