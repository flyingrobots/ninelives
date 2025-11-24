//! Clock abstractions used by circuit breakers and other time-based policies.

use std::time::Instant;

/// Clock abstraction so timing can be faked in tests.
pub trait Clock: Send + Sync + std::fmt::Debug {
    fn now_millis(&self) -> u64;
}

/// Monotonic clock backed by `Instant::now()`.
///
/// Notes: resets when the process restarts; use a wall-clock-based implementation if you
/// need timing that survives restarts.
#[derive(Debug, Clone)]
pub struct MonotonicClock {
    start: Instant,
}

impl Default for MonotonicClock {
    fn default() -> Self {
        Self { start: Instant::now() }
    }
}

impl Clock for MonotonicClock {
    fn now_millis(&self) -> u64 {
        u64::try_from(self.start.elapsed().as_millis()).unwrap_or(u64::MAX)
    }
}
