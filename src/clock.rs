//! Clock abstractions used by circuit breakers and other time-based policies.

use std::time::Instant;

/// Thread-safe time source abstraction.
///
/// Implementers must document whether the origin is wall-clock (e.g., UNIX epoch)
/// or monotonic process time. The return value is milliseconds since that origin.
/// Calls must be safe concurrently (`Send + Sync`). Overflow must not silently wrap;
/// prefer saturating or panicking semantics and document the choice.
pub trait Clock: Send + Sync + std::fmt::Debug {
    /// Current time in milliseconds relative to the implementer's origin.
    /// Implementations should state monotonicity guarantees and overflow policy.
    fn now_millis(&self) -> u64;
}

/// Monotonic clock backed by `Instant::now()`.
///
/// Clones share the same epoch (instant captured at creation). Independently created
/// instances have different epochs and their readings are not directly comparable.
/// Resets on process restart; use a wall-clock clock if you need cross-restart continuity.
#[derive(Debug, Clone)]
pub struct MonotonicClock {
    start: Instant,
}

impl MonotonicClock {
    /// Create a new monotonic clock starting at `Instant::now()`.
    pub fn new() -> Self {
        Self { start: Instant::now() }
    }
}

impl Default for MonotonicClock {
    fn default() -> Self {
        Self::new()
    }
}

impl Clock for MonotonicClock {
    /// Returns milliseconds elapsed since this instance's epoch.
    ///
    /// On the theoretical overflow (>584 million years), saturates to `u64::MAX`.
    fn now_millis(&self) -> u64 {
        u64::try_from(self.start.elapsed().as_millis()).unwrap_or(u64::MAX)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn monotonic_non_decreasing() {
        let clock = MonotonicClock::new();
        let first = clock.now_millis();
        let second = clock.now_millis();
        assert!(second >= first);
    }

    #[test]
    fn clones_share_epoch() {
        let clock = MonotonicClock::new();
        let clone = clock.clone();
        let a = clock.now_millis();
        let b = clone.now_millis();
        let diff = a.abs_diff(b);
        assert!(diff < 50, "Clones differ by {}ms", diff);
        thread::sleep(Duration::from_millis(5));
        let a2 = clock.now_millis();
        let b2 = clone.now_millis();
        let diff2 = a2.abs_diff(b2);
        assert!(diff2 < 50, "Clones differ by {}ms after sleep", diff2);
    }

    #[test]
    fn independent_epochs_differ() {
        let a = MonotonicClock::new();
        thread::sleep(Duration::from_millis(2));
        let b = MonotonicClock::new();
        let a_now = a.now_millis();
        let b_now = b.now_millis();
        assert!(
            a_now > b_now,
            "Expected a ({a_now}ms) > b ({b_now}ms) due to 2ms sleep between creations"
        );
    }

    #[test]
    fn trait_object_usage() {
        let clock: Box<dyn Clock> = Box::new(MonotonicClock::new());
        let _ = clock.now_millis();
    }

    #[test]
    fn send_sync_across_threads() {
        let clock = Arc::new(MonotonicClock::new());
        let mut handles = vec![];
        for _ in 0..4 {
            let c = clock.clone();
            handles.push(thread::spawn(move || {
                let _ = c.now_millis();
            }));
        }
        for h in handles {
            h.join().unwrap();
        }
    }
}
