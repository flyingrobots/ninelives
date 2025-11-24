//! Backoff strategies for retry policies

use std::time::Duration;

/// Backoff strategy for retries
#[derive(Debug, Clone)]
pub enum Backoff {
    /// Fixed delay between retries
    Constant { delay: Duration },
    /// Linearly increasing delay
    Linear { base: Duration },
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
        Backoff::Linear { base }
    }

    /// Create an exponential backoff strategy
    pub fn exponential(base: Duration) -> Self {
        Backoff::Exponential { base, max: None }
    }

    /// Set a maximum delay for exponential backoff
    pub fn with_max(mut self, max: Duration) -> Self {
        if let Backoff::Exponential { max: ref mut m, .. } = self {
            *m = Some(max);
        }
        self
    }

    /// Calculate the delay for a given attempt number (1-indexed)
    pub fn delay(&self, attempt: usize) -> Duration {
        match self {
            Backoff::Constant { delay } => *delay,
            Backoff::Linear { base } => {
                // Use checked_mul to prevent overflow
                base.checked_mul(attempt as u32)
                    .unwrap_or(Duration::from_secs(u64::MAX))
            }
            Backoff::Exponential { base, max } => {
                // Calculate 2^(attempt-1) with overflow protection
                let exponent = (attempt.saturating_sub(1)) as u32;
                let multiplier = 2u32.saturating_pow(exponent);

                let exp_delay = base.checked_mul(multiplier)
                    .unwrap_or(Duration::from_secs(u64::MAX));

                if let Some(max) = max {
                    exp_delay.min(*max)
                } else {
                    exp_delay
                }
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
    fn exponential_backoff_doubles_each_time() {
        let backoff = Backoff::exponential(Duration::from_millis(100));
        assert_eq!(backoff.delay(1), Duration::from_millis(100));  // 100 * 2^0
        assert_eq!(backoff.delay(2), Duration::from_millis(200));  // 100 * 2^1
        assert_eq!(backoff.delay(3), Duration::from_millis(400));  // 100 * 2^2
        assert_eq!(backoff.delay(4), Duration::from_millis(800));  // 100 * 2^3
        assert_eq!(backoff.delay(5), Duration::from_millis(1600)); // 100 * 2^4
    }

    #[test]
    fn exponential_backoff_respects_max() {
        let backoff = Backoff::exponential(Duration::from_millis(100))
            .with_max(Duration::from_secs(1));

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
        // Attempt 64 would overflow u32, should saturate
        let delay = backoff.delay(64);
        assert!(delay > Duration::from_secs(1_000_000)); // Very large but not panicking
    }

    #[test]
    fn linear_backoff_handles_overflow() {
        let backoff = Backoff::linear(Duration::from_secs(u64::MAX / 2));
        // Should saturate to max duration instead of panicking
        let delay = backoff.delay(10);
        assert!(delay >= Duration::from_secs(u64::MAX / 2));
    }

    #[test]
    fn with_max_only_affects_exponential() {
        let constant = Backoff::constant(Duration::from_secs(5))
            .with_max(Duration::from_secs(1));
        // Shouldn't affect constant backoff
        assert_eq!(constant.delay(1), Duration::from_secs(5));

        let linear = Backoff::linear(Duration::from_secs(5))
            .with_max(Duration::from_secs(1));
        // Shouldn't affect linear backoff
        assert_eq!(linear.delay(2), Duration::from_secs(10));
    }
}
