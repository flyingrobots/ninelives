//! Abstraction for sleeping/waiting
//!
//! Enables fast, deterministic tests without real time delays

use async_trait::async_trait;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Abstraction for sleeping/waiting
#[async_trait]
pub trait Sleeper: Send + Sync + std::fmt::Debug {
    async fn sleep(&self, duration: Duration);
}

/// Production sleeper using tokio runtime
#[derive(Debug, Default, Clone, Copy)]
pub struct TokioSleeper;

#[async_trait]
impl Sleeper for TokioSleeper {
    async fn sleep(&self, duration: Duration) {
        tokio::time::sleep(duration).await
    }
}

/// Test sleeper that doesn't actually sleep
#[derive(Debug, Default, Clone, Copy)]
pub struct InstantSleeper;

#[async_trait]
impl Sleeper for InstantSleeper {
    async fn sleep(&self, _duration: Duration) {
        // no-op
    }
}

/// Test sleeper that tracks all sleep calls
#[derive(Debug, Clone)]
pub struct TrackingSleeper {
    calls: Arc<Mutex<Vec<Duration>>>,
}

impl TrackingSleeper {
    pub fn new() -> Self {
        Self { calls: Arc::new(Mutex::new(Vec::new())) }
    }

    /// Number of recorded sleep calls.
    pub fn calls(&self) -> usize {
        self.calls.lock().expect("TrackingSleeper.calls: mutex poisoned").len()
    }

    /// Get a recorded call duration by index, if present.
    pub fn call_at(&self, index: usize) -> Option<Duration> {
        self.calls.lock().expect("TrackingSleeper.call_at: mutex poisoned").get(index).copied()
    }

    pub fn clear(&self) {
        self.calls.lock().expect("TrackingSleeper.clear: mutex poisoned").clear();
    }
}
#[async_trait]
impl Sleeper for TrackingSleeper {
    async fn sleep(&self, duration: Duration) {
        self.calls.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).push(duration);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn instant_sleeper_doesnt_sleep() {
        let sleeper = InstantSleeper;
        let start = std::time::Instant::now();
        sleeper.sleep(Duration::from_secs(10)).await;
        let elapsed = start.elapsed();
        // Should complete almost instantly
        assert!(elapsed < Duration::from_millis(100));
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

    #[tokio::test]
    async fn tokio_sleeper_actually_sleeps() {
        let sleeper = TokioSleeper;
        let start = std::time::Instant::now();
        sleeper.sleep(Duration::from_millis(50)).await;
        let elapsed = start.elapsed();
        // Should take at least the requested duration
        assert!(elapsed >= Duration::from_millis(45)); // Small tolerance for timing jitter
    }
}
