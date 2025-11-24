//! Jitter strategies to prevent thundering herd

use rand::Rng;
use std::time::Duration;

/// Jitter strategy for randomizing retry delays
#[derive(Debug, Clone)]
pub enum Jitter {
    /// No jitter - use exact backoff delay
    None,
    /// Full jitter: random between 0 and delay
    Full,
    /// Equal jitter: random between delay/2 and delay
    Equal,
    /// Decorrelated jitter: AWS-style with state
    Decorrelated { base: Duration, max: Duration },
}

impl Jitter {
    /// Create a full jitter strategy
    pub fn full() -> Self {
        Jitter::Full
    }

    /// Create an equal jitter strategy
    pub fn equal() -> Self {
        Jitter::Equal
    }

    /// Create a decorrelated jitter strategy
    pub fn decorrelated(base: Duration, max: Duration) -> Self {
        Jitter::Decorrelated { base, max }
    }

    /// Apply jitter to a delay duration
    pub fn apply(&self, delay: Duration) -> Duration {
        match self {
            Jitter::None => delay,
            Jitter::Full => {
                let millis = delay.as_millis() as u64;
                if millis == 0 {
                    return Duration::from_millis(0);
                }
                let jittered = rand::thread_rng().gen_range(0..=millis);
                Duration::from_millis(jittered)
            }
            Jitter::Equal => {
                let millis = delay.as_millis() as u64;
                if millis == 0 {
                    return Duration::from_millis(0);
                }
                let half = millis / 2;
                let jittered = rand::thread_rng().gen_range(half..=millis);
                Duration::from_millis(jittered)
            }
            Jitter::Decorrelated { base, max } => {
                // Decorrelated jitter: sleep = min(cap, random(base, sleep * 3))
                // For simplicity, we use the delay as previous sleep
                let base_millis = base.as_millis() as u64;
                let delay_millis = delay.as_millis() as u64;
                let max_millis = max.as_millis() as u64;

                let upper = delay_millis.saturating_mul(3);
                let range_max = upper.min(max_millis);

                if base_millis >= range_max {
                    return Duration::from_millis(base_millis);
                }

                let jittered = rand::thread_rng().gen_range(base_millis..=range_max);
                Duration::from_millis(jittered)
            }
        }
    }

    /// Apply jitter with a custom RNG (for testing)
    pub fn apply_with_rng<R: Rng>(&self, delay: Duration, rng: &mut R) -> Duration {
        match self {
            Jitter::None => delay,
            Jitter::Full => {
                let millis = delay.as_millis() as u64;
                if millis == 0 {
                    return Duration::from_millis(0);
                }
                let jittered = rng.gen_range(0..=millis);
                Duration::from_millis(jittered)
            }
            Jitter::Equal => {
                let millis = delay.as_millis() as u64;
                if millis == 0 {
                    return Duration::from_millis(0);
                }
                let half = millis / 2;
                let jittered = rng.gen_range(half..=millis);
                Duration::from_millis(jittered)
            }
            Jitter::Decorrelated { base, max } => {
                let base_millis = base.as_millis() as u64;
                let delay_millis = delay.as_millis() as u64;
                let max_millis = max.as_millis() as u64;

                let upper = delay_millis.saturating_mul(3);
                let range_max = upper.min(max_millis);

                if base_millis >= range_max {
                    return Duration::from_millis(base_millis);
                }

                let jittered = rng.gen_range(base_millis..=range_max);
                Duration::from_millis(jittered)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    #[test]
    fn none_jitter_returns_exact_delay() {
        let jitter = Jitter::None;
        let delay = Duration::from_secs(1);
        assert_eq!(jitter.apply(delay), delay);
    }

    #[test]
    fn full_jitter_is_between_zero_and_delay() {
        let jitter = Jitter::full();
        let delay = Duration::from_secs(1);

        // Test multiple times to ensure randomness
        for _ in 0..100 {
            let jittered = jitter.apply(delay);
            assert!(jittered <= delay);
            assert!(jittered >= Duration::from_millis(0));
        }
    }

    #[test]
    fn equal_jitter_is_between_half_and_delay() {
        let jitter = Jitter::equal();
        let delay = Duration::from_secs(1);
        let half = Duration::from_millis(500);

        // Test multiple times
        for _ in 0..100 {
            let jittered = jitter.apply(delay);
            assert!(jittered <= delay);
            assert!(jittered >= half);
        }
    }

    #[test]
    fn full_jitter_with_deterministic_rng() {
        let jitter = Jitter::full();
        let delay = Duration::from_millis(1000);
        let mut rng = StdRng::seed_from_u64(42);

        let jittered = jitter.apply_with_rng(delay, &mut rng);
        assert!(jittered <= delay);
        assert!(jittered < Duration::from_millis(1000)); // Should be randomized
    }

    #[test]
    fn equal_jitter_with_deterministic_rng() {
        let jitter = Jitter::equal();
        let delay = Duration::from_millis(1000);
        let mut rng = StdRng::seed_from_u64(42);

        let jittered = jitter.apply_with_rng(delay, &mut rng);
        assert!(jittered >= Duration::from_millis(500));
        assert!(jittered <= delay);
    }

    #[test]
    fn decorrelated_jitter_respects_bounds() {
        let jitter = Jitter::decorrelated(Duration::from_millis(100), Duration::from_secs(10));
        let delay = Duration::from_secs(1);

        for _ in 0..100 {
            let jittered = jitter.apply(delay);
            assert!(jittered >= Duration::from_millis(100)); // >= base
            assert!(jittered <= Duration::from_secs(10)); // <= max
        }
    }

    #[test]
    fn jitter_handles_zero_delay() {
        assert_eq!(
            Jitter::full().apply(Duration::from_millis(0)),
            Duration::from_millis(0)
        );
        assert_eq!(
            Jitter::equal().apply(Duration::from_millis(0)),
            Duration::from_millis(0)
        );
    }

    #[test]
    fn decorrelated_jitter_caps_at_max() {
        let jitter = Jitter::decorrelated(Duration::from_secs(1), Duration::from_secs(5));
        let huge_delay = Duration::from_secs(100);

        for _ in 0..50 {
            let jittered = jitter.apply(huge_delay);
            assert!(jittered <= Duration::from_secs(5));
        }
    }
}
