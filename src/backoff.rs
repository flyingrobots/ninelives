//! Backoff strategies for retry policies.
//!
//! Provides constant, linear, and exponential strategies with optional caps. Attempt semantics:
//! attempt index `0` represents the initial call (no delay), and retries start at `attempt = 1`.
//! Delays saturate at a documented maximum to avoid overflow.
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
//! greater than `u32::MAX` are clamped to `u32::MAX` when computing multipliers.

use std::time::Duration;

/// Maximum delay used when calculations overflow (1 day).
pub const MAX_BACKOFF: Duration = Duration::from_secs(24 * 60 * 60);

/// Errors returned by backoff configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackoffError {
    ConstantDoesNotSupportMax,
    MaxMustBePositive,
    MaxLessThanBase { base: Duration, max: Duration },
}

impl std::fmt::Display for BackoffError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BackoffError::ConstantDoesNotSupportMax => {
                write!(f, "with_max is only valid for Linear or Exponential backoff")
            }
            BackoffError::MaxMustBePositive => write!(f, "max must be greater than zero"),
            BackoffError::MaxLessThanBase { base, max } => {
                write!(f, "max ({:?}) must be >= base ({:?})", max, base)
            }
        }
    }
}

impl std::error::Error for BackoffError {}

/// Backoff strategy for retries
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Backoff {
    /// Fixed delay between retries
    Constant { delay: Duration },
    /// Linearly increasing delay with optional cap
    Linear { base: Duration, max: Option<Duration> },
    /// Exponentially increasing delay with optional cap
    Exponential { base: Duration, max: Option<Duration> },
}

impl Backoff {
    /// Create a constant backoff strategy
    pub fn constant(delay: Duration) -> Self {
        Backoff::Constant { delay }
    }

    /// Create a linear backoff strategy
    pub fn linear(base: Duration) -> Self {
        Backoff::Linear { base, max: None }
    }

    /// Create an exponential backoff strategy
    pub fn exponential(base: Duration) -> Self {
        Backoff::Exponential { base, max: None }
    }

    /// Set a maximum delay for the backoff (linear or exponential).
    /// Returns an error if called on `Constant`, if `max` is zero, or if `max < base`.
    pub fn with_max(mut self, max: Duration) -> Result<Self, BackoffError> {
        if max.is_zero() {
            return Err(BackoffError::MaxMustBePositive);
        }
        match &mut self {
            Backoff::Exponential { max: existing, base } => {
                if max < *base {
                    return Err(BackoffError::MaxLessThanBase { base: *base, max });
                }
                *existing = Some(max);
                Ok(self)
            }
            Backoff::Linear { max: existing, base } => {
                if max < *base {
                    return Err(BackoffError::MaxLessThanBase { base: *base, max });
                }
                *existing = Some(max);
                Ok(self)
            }
            Backoff::Constant { .. } => Err(BackoffError::ConstantDoesNotSupportMax),
        }
    }

    /// Calculate the delay for a given attempt number (0-based; 0 = initial call, no delay).
    pub fn delay(&self, attempt: usize) -> Duration {
        if attempt == 0 {
            return Duration::from_millis(0);
        }

        let clamp_overflow = |d: Option<Duration>| d.unwrap_or(MAX_BACKOFF);

        match self {
            Backoff::Constant { delay } => *delay,
            Backoff::Linear { base, max } => {
                // Duration::checked_mul takes u32; clamp attempt to u32::MAX to prevent truncation/overflow.
                let attempt_u32 = attempt.min(u32::MAX as usize) as u32;
                let linear = clamp_overflow(base.checked_mul(attempt_u32));
                max.map(|m| linear.min(m)).unwrap_or(linear)
            }
            Backoff::Exponential { base, max } => {
                // Calculate 2^(attempt-1) with overflow protection; attempt >=1 here.
                let exponent = attempt.saturating_sub(1).min(u32::MAX as usize) as u32;
                let multiplier = 2u128.saturating_pow(exponent as u32);
                let base_nanos = base.as_nanos().saturating_mul(multiplier);
                let exp_delay = Duration::from_nanos(base_nanos.min(MAX_BACKOFF.as_nanos()) as u64);
                max.map(|m| exp_delay.min(m)).unwrap_or(exp_delay)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constant_backoff_returns_same_delay() {
        let backoff = Backoff::constant(Duration::from_secs(1));
        assert_eq!(backoff.delay(1), Duration::from_secs(1));
        assert_eq!(backoff.delay(2), Duration::from_secs(1));
        assert_eq!(backoff.delay(100), Duration::from_secs(1));
    }

    #[test]
    fn linear_backoff_increases_linearly() {
        let backoff = Backoff::linear(Duration::from_millis(100));
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
    fn with_max_on_constant_errors() {
        let constant = Backoff::constant(Duration::from_secs(5)).with_max(Duration::from_secs(1));
        assert!(matches!(constant, Err(BackoffError::ConstantDoesNotSupportMax)));
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
}
