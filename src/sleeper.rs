//! Abstraction for sleeping/waiting
//!
//! Implementations provided:
//! - `TokioSleeper`: production async sleeps via `tokio::time::sleep` (requires an active Tokio
//!   runtime; using under other runtimes may panic).
//! - `InstantSleeper`: test helper that returns immediately (no real delay).
//! - `TrackingSleeper`: test helper that records every requested sleep for assertions.
//!
//! Use `TokioSleeper` in production stacks; prefer `InstantSleeper` or `TrackingSleeper` in
//! tests to avoid wall-clock delays while still validating retry/backoff behavior. Sleeps are
//! expected to be cancellation-safe (dropping the future cancels the timer), accept `Duration::ZERO`
//! (treated as immediate), and handle very large durations by clamping or saturating without panic.

use async_trait::async_trait;
use std::fmt::Debug;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Contract for sleeping/waiting.
///
/// Requirements for implementors:
/// - Accept any `Duration`, treating `Duration::ZERO` as an immediate no-op.
/// - Large durations must not panic; clamp/saturate if needed.
/// - Must be `Send + Sync + Debug` and safe to call concurrently.
/// - Should be cancellation-safe: dropping the returned future cancels the sleep without side
///   effects; no panics on normal use.
#[async_trait]
pub trait Sleeper: Send + Sync + Debug {
    /// Sleep for the requested duration; may return early if the future is dropped.
    async fn sleep(&self, duration: Duration);
}

/// Production sleeper using tokio runtime.
#[derive(Debug, Default, Clone, Copy)]
pub struct TokioSleeper;

#[async_trait]
impl Sleeper for TokioSleeper {
    async fn sleep(&self, duration: Duration) {
        tokio::time::sleep(duration).await
    }
}

/// Test sleeper that doesn't actually sleep.
#[derive(Debug, Default, Clone, Copy)]
pub struct InstantSleeper;

#[async_trait]
impl Sleeper for InstantSleeper {
    async fn sleep(&self, _duration: Duration) {
        // no-op
    }
}

/// Test sleeper that tracks all sleep calls. Clones share the same recorded history via `Arc`, so
/// any clone observing or pushing sleeps sees the combined timeline.
#[derive(Debug, Clone)]
pub struct TrackingSleeper {
    calls: Arc<Mutex<Vec<Duration>>>,
}

impl TrackingSleeper {
    /// Create a new tracking sleeper that records all calls.
    pub fn new() -> Self {
        Self { calls: Arc::new(Mutex::new(Vec::new())) }
    }

    /// Number of recorded sleep calls.
    pub fn calls(&self) -> usize {
        self.calls.lock().unwrap_or_else(|e| e.into_inner()).len()
    }

    /// Get a recorded call duration by index, if present.
    pub fn call_at(&self, index: usize) -> Option<Duration> {
        self.calls.lock().unwrap_or_else(|e| e.into_inner()).get(index).copied()
    }

    /// Returns a clone of all recorded sleep calls.
    /// **Note:** clones the full vector; prefer `calls()` and `call_at()` when you only need counts
    /// or specific entries.
    pub fn all_calls(&self) -> Vec<Duration> {
        self.calls.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    /// Remove all recorded sleep calls.
    /// Recovers from a poisoned lock by taking ownership of the inner vector and clearing it.
    pub fn clear(&self) {
        self.calls.lock().unwrap_or_else(|e| e.into_inner()).clear();
    }
}

impl Default for TrackingSleeper {
    fn default() -> Self {
        Self::new()
    }
}
#[async_trait]
impl Sleeper for TrackingSleeper {
    async fn sleep(&self, duration: Duration) {
        // Records immediately; does not yield to the scheduler. For a yielding variant, wrap
        // this in `tokio::task::yield_now().await` or use `TokioSleeper`.
        self.calls.lock().unwrap_or_else(|e| e.into_inner()).push(duration);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::future::join_all;

    #[tokio::test]
    async fn instant_sleeper_doesnt_sleep() {
        let sleeper = InstantSleeper;
        let start = std::time::Instant::now();
        sleeper.sleep(Duration::from_secs(10)).await;
        let elapsed = start.elapsed();
        // Should complete almost instantly
        assert!(elapsed < Duration::from_millis(10));
    }

    #[tokio::test]
    async fn tracking_sleeper_records_calls() {
        let sleeper = TrackingSleeper::new();

        sleeper.sleep(Duration::from_millis(100)).await;
        sleeper.sleep(Duration::from_millis(200)).await;
        sleeper.sleep(Duration::from_millis(400)).await;

        assert_eq!(sleeper.calls(), 3);
        assert_eq!(sleeper.call_at(0).unwrap(), Duration::from_millis(100));
        assert_eq!(sleeper.call_at(1).unwrap(), Duration::from_millis(200));
        assert_eq!(sleeper.call_at(2).unwrap(), Duration::from_millis(400));
    }

    #[tokio::test]
    async fn tracking_sleeper_can_clear() {
        let sleeper = TrackingSleeper::new();

        sleeper.sleep(Duration::from_millis(100)).await;
        assert_eq!(sleeper.calls(), 1);

        sleeper.clear();
        assert_eq!(sleeper.calls(), 0);

        sleeper.sleep(Duration::from_millis(200)).await;
        assert_eq!(sleeper.calls(), 1);
        assert_eq!(sleeper.call_at(0).unwrap(), Duration::from_millis(200));
    }

    #[tokio::test(start_paused = true)]
    async fn tokio_sleeper_actually_sleeps() {
        use std::future::Future;

        let start = tokio::time::Instant::now();
        let sleep = TokioSleeper.sleep(Duration::from_millis(50));
        tokio::pin!(sleep);
        // Poll once to register the timer before advancing the mocked clock
        let waker = futures::task::noop_waker();
        let mut cx = std::task::Context::from_waker(&waker);
        assert!(sleep.as_mut().poll(&mut cx).is_pending());
        tokio::time::advance(Duration::from_millis(50)).await;
        let elapsed = start.elapsed();
        assert_eq!(elapsed, Duration::from_millis(50));
        sleep.await;
        let elapsed_after = start.elapsed();
        assert_eq!(elapsed_after, Duration::from_millis(50));
    }

    #[tokio::test]
    async fn tracking_sleeper_call_at_out_of_range() {
        let sleeper = TrackingSleeper::new();
        sleeper.sleep(Duration::from_millis(10)).await;
        assert!(sleeper.call_at(1).is_none());
        assert!(sleeper.call_at(999).is_none());
    }

    #[tokio::test]
    async fn tracking_sleeper_concurrent_recording() {
        let sleeper = TrackingSleeper::new();
        let a = sleeper.clone();
        let b = sleeper.clone();

        let h1 = tokio::spawn(async move { a.sleep(Duration::from_millis(1)).await });
        let h2 = tokio::spawn(async move { b.sleep(Duration::from_millis(2)).await });

        let results = join_all([h1, h2]).await;
        for r in results {
            r.expect("task panicked");
        }

        assert_eq!(sleeper.calls(), 2);
        // Order is not guaranteed, but both durations should be present.
        let mut recorded = sleeper.all_calls();
        recorded.sort();
        assert_eq!(recorded, vec![Duration::from_millis(1), Duration::from_millis(2)]);
    }

    #[tokio::test]
    async fn tracking_sleeper_clone_shares_state() {
        let sleeper = TrackingSleeper::new();
        let clone = sleeper.clone();

        clone.sleep(Duration::from_millis(5)).await;

        assert_eq!(sleeper.calls(), 1);
        assert_eq!(sleeper.call_at(0), Some(Duration::from_millis(5)));
    }

    #[tokio::test]
    async fn tracking_sleeper_default_matches_new() {
        let sleeper = TrackingSleeper::default();
        sleeper.sleep(Duration::from_millis(1)).await;
        assert_eq!(sleeper.calls(), 1);
    }

    #[tokio::test]
    async fn tracking_sleeper_all_calls_returns_owned_copy() {
        let sleeper = TrackingSleeper::new();
        sleeper.sleep(Duration::from_millis(10)).await;
        sleeper.sleep(Duration::from_millis(20)).await;
        let recorded = sleeper.all_calls();
        assert_eq!(recorded, vec![Duration::from_millis(10), Duration::from_millis(20)]);
    }
}
