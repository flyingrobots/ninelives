//! Jitter strategies to prevent thundering herd
//!
//! When to use which strategy:
//! - `None`: deterministic retries for tests or tightly controlled workflows.
//! - `Full`: uniform in `[0, delay]`, good default to spread load.
//! - `Equal`: uniform in `[delay/2, delay]`, keeps a floor while adding randomness.
//! - `Decorrelated`: AWS-style decorrelated jitter that grows based on previous sleep; use the
//!   stateful API (`apply_stateful`) because the `delay` argument is not used.
//!
//! Notes:
//! - RNG: uses `rand`'s thread-local RNG by default; deterministic RNGs can be injected via `apply_with_rng`.
//! - Precision: millisecond conversions saturate to `u64::MAX` to avoid panics on very large durations.
//! - Decorrelated jitter here is stateful; it tracks the previous sleep internally. Use\n//!   `apply_stateful` for the decorrelated variant so you don’t pass a meaningless delay. Configure\n//!   its starting magnitude with `Jitter::decorrelated(base, max)` and your backoff choice.
//! - Thread-safety: `Decorrelated` stores its state in an `AtomicU64`; sharing the same handle
//!   across threads is safe. Cloning a decorrelated jitter copies the state, creating an independent
//!   handle.
//! Invariants:
//! - None leaves delay unchanged.
//! - Full/Equal always return values inside their documented ranges.
//! - Decorrelated outputs are within `[max(prev, base), min(max, 3*prev)]`, never negative, and do
//!   not regress below the previous value.
//!
//! Example:
//! ```rust
//! use ninelives::{Backoff, Jitter};
//! use std::time::Duration;
//!
//! let jitter = Jitter::decorrelated(Duration::from_millis(100), Duration::from_secs(5)).unwrap();
//! let backoff = Backoff::exponential(Duration::from_millis(100));
//! // decorrelated jitter will evolve from its internal state (starting at base) rather than the backoff-provided delay
//! ```

use rand::{rng, Rng};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

#[derive(Debug)]
/// Internal state for decorrelated jitter; fields are private to enforce validation.
pub struct DecorrelatedConfig {
    base: Duration,
    max: Duration,
    previous: AtomicU64,
}

impl Clone for DecorrelatedConfig {
    fn clone(&self) -> Self {
        let prev = self.previous.load(Ordering::Relaxed);
        Self { base: self.base, max: self.max, previous: AtomicU64::new(prev) }
    }
}

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
    Decorrelated(DecorrelatedConfig),
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
    pub fn decorrelated(base: Duration, max: Duration) -> Result<Self, &'static str> {
        if base > max {
            return Err("decorrelated jitter: base must not exceed max");
        }

        let base_millis = Self::as_millis_saturated(base);

        Ok(Jitter::Decorrelated(DecorrelatedConfig {
            base,
            max,
            previous: AtomicU64::new(base_millis),
        }))
    }

    /// Apply jitter to a delay duration
    pub fn apply(&self, delay: Duration) -> Duration {
        match self {
            Jitter::Decorrelated(_) => {
                panic!("use apply_stateful() for decorrelated jitter to avoid ignoring delay")
            }
            _ => {
                let mut rng = rng();
                self.apply_internal(delay, &mut rng)
            }
        }
    }

    /// Apply jitter for the decorrelated strategy, which uses internal state and ignores the passed delay.
    pub fn apply_stateful(&self) -> Duration {
        match self {
            Jitter::Decorrelated(config) => {
                let base_millis = Self::as_millis_saturated(config.base);
                let max_millis = Self::as_millis_saturated(config.max);
                let mut rng = rng();

                loop {
                    let prev_millis = config.previous.load(Ordering::Acquire);
                    let upper = prev_millis.saturating_mul(3).min(max_millis);
                    let lower = prev_millis.max(base_millis).min(upper);
                    let jittered = rng.random_range(lower..=upper);
                    if config
                        .previous
                        .compare_exchange(
                            prev_millis,
                            jittered,
                            Ordering::AcqRel,
                            Ordering::Acquire,
                        )
                        .is_ok()
                    {
                        return Duration::from_millis(jittered);
                    }
                }
            }
            _ => panic!("apply_stateful only valid for decorrelated jitter"),
        }
    }

    /// Apply jitter with a custom RNG (for testing)
    pub fn apply_with_rng<R: Rng>(&self, delay: Duration, rng: &mut R) -> Duration {
        self.apply_internal(delay, rng)
    }

    fn as_millis_saturated(duration: Duration) -> u64 {
        duration.as_millis().try_into().unwrap_or(u64::MAX) // Saturate extremely large durations
    }

    fn apply_internal<R: Rng>(&self, delay: Duration, rng: &mut R) -> Duration {
        match self {
            Jitter::None => delay,
            Jitter::Full => {
                let millis = Self::as_millis_saturated(delay);
                let jittered = rng.random_range(0..=millis);
                Duration::from_millis(jittered)
            }
            Jitter::Equal => {
                let millis = Self::as_millis_saturated(delay);
                let half = millis / 2;
                let jittered = rng.random_range(half..=millis);
                Duration::from_millis(jittered)
            }
            Jitter::Decorrelated(config) => {
                // Decorrelated jitter: sleep = min(max, random(base, prev_sleep * 3))
                let base_millis = Self::as_millis_saturated(config.base);
                let max_millis = Self::as_millis_saturated(config.max);

                let prev_millis = config.previous.load(Ordering::Relaxed);

                // upper bound grows from previous sleep, capped by max
                let upper = prev_millis.saturating_mul(3).min(max_millis);
                // lower bound keeps floor at base and does not regress from previous
                let lower = prev_millis.max(base_millis).min(upper);

                let jittered = rng.random_range(lower..=upper);

                config.previous.store(jittered, Ordering::Relaxed);
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
    use std::sync::Arc;

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
        let jitter =
            Jitter::decorrelated(Duration::from_millis(100), Duration::from_secs(10)).unwrap();

        for _ in 0..100 {
            let jittered = jitter.apply_stateful();
            assert!(jittered >= Duration::from_millis(100)); // >= base
            assert!(jittered <= Duration::from_secs(10)); // <= max
        }
    }

    #[test]
    fn jitter_handles_zero_delay() {
        assert_eq!(Jitter::full().apply(Duration::from_millis(0)), Duration::from_millis(0));
        assert_eq!(Jitter::equal().apply(Duration::from_millis(0)), Duration::from_millis(0));
        let decorrelated =
            Jitter::decorrelated(Duration::from_millis(100), Duration::from_secs(10)).unwrap();
        let first = decorrelated.apply_stateful();
        assert!(first >= Duration::from_millis(100));
    }

    #[test]
    fn decorrelated_jitter_caps_at_max() {
        let jitter = Jitter::decorrelated(Duration::from_secs(1), Duration::from_secs(5)).unwrap();

        for _ in 0..50 {
            let jittered = jitter.apply_stateful();
            assert!(jittered <= Duration::from_secs(5));
            assert!(jittered >= Duration::from_secs(1));
        }
    }

    #[test]
    fn decorrelated_constructor_rejects_invalid_bounds() {
        let err = Jitter::decorrelated(Duration::from_secs(5), Duration::from_secs(1))
            .expect_err("base > max should error");
        assert_eq!(err, "decorrelated jitter: base must not exceed max");
    }

    #[test]
    fn decorrelated_uses_previous_sleep_statefully() {
        let mut rng = StdRng::seed_from_u64(123);
        let jitter =
            Jitter::decorrelated(Duration::from_millis(100), Duration::from_secs(10)).unwrap();

        // First call: prev = base, so upper = min(max, base*3) = 300ms
        let first = jitter.apply_with_rng(Duration::from_secs(1), &mut rng);
        assert!(first >= Duration::from_millis(100));
        assert!(first <= Duration::from_millis(300));

        // Second call should use previous jittered sleep as seed for growth
        let expected_upper =
            std::cmp::min(first.as_millis().saturating_mul(3), Duration::from_secs(10).as_millis())
                as u64;
        let second = jitter.apply_with_rng(Duration::from_secs(1), &mut rng);
        assert!(second.as_millis() as u64 >= 100);
        assert!(second.as_millis() as u64 <= expected_upper);
    }

    #[test]
    fn decorrelated_never_regresses() {
        let jitter =
            Jitter::decorrelated(Duration::from_millis(10), Duration::from_millis(200)).unwrap();
        let mut prev = Duration::from_millis(0);
        for _ in 0..200 {
            let next = jitter.apply_stateful();
            assert!(next >= prev, "decorrelated jitter regressed");
            assert!(next <= Duration::from_millis(200));
            prev = next;
        }
    }

    #[test]
    fn saturates_large_durations_without_panicking() {
        let huge = Duration::from_secs(u64::MAX);
        let jitter = Jitter::full();
        let mut rng = StdRng::seed_from_u64(999);

        let jittered = jitter.apply_with_rng(huge, &mut rng);
        assert!(jittered <= Duration::from_millis(u64::MAX));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn decorrelated_state_progresses_monotonically_under_contention() {
        let jitter = Arc::new(
            Jitter::decorrelated(Duration::from_millis(5), Duration::from_secs(1)).unwrap(),
        );

        let tasks: Vec<_> = (0..64)
            .map(|_| {
                let j = jitter.clone();
                tokio::spawn(async move {
                    let mut last = Duration::from_millis(0);
                    for _ in 0..200 {
                        let next = j.apply_stateful();
                        assert!(
                            next >= last,
                            "decorrelated jitter regressed from {:?} to {:?}",
                            last,
                            next
                        );
                        assert!(next >= Duration::from_millis(5));
                        assert!(next <= Duration::from_secs(1));
                        last = next;
                    }
                })
            })
            .collect();

        for task in tasks {
            task.await.unwrap();
        }
    }
}
