//! Abstraction for sleeping/waiting
//!
//! Enables fast, deterministic tests without real time delays

use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Abstraction for sleeping/waiting
pub trait Sleeper: Send + Sync + std::fmt::Debug {
    fn sleep(&self, duration: Duration) -> Pin<Box<dyn Future<Output = ()> + Send>>;
}

/// Production sleeper using tokio runtime
#[derive(Debug, Default, Clone, Copy)]
pub struct TokioSleeper;

impl Sleeper for TokioSleeper {
    fn sleep(&self, duration: Duration) -> Pin<Box<dyn Future<Output = ()> + Send>> {
        Box::pin(tokio::time::sleep(duration))
    }
}

/// Test sleeper that doesn't actually sleep
#[derive(Debug, Default, Clone, Copy)]
pub struct InstantSleeper;

impl Sleeper for InstantSleeper {
    fn sleep(&self, _duration: Duration) -> Pin<Box<dyn Future<Output = ()> + Send>> {
        Box::pin(async {})
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

    pub fn calls(&self) -> Vec<Duration> {
        self.calls.lock().unwrap().clone()
    }

    pub fn clear(&self) {
        self.calls.lock().unwrap().clear();
    }
}

impl Default for TrackingSleeper {
    fn default() -> Self {
        Self::new()
    }
}

impl Sleeper for TrackingSleeper {
    fn sleep(&self, duration: Duration) -> Pin<Box<dyn Future<Output = ()> + Send>> {
        self.calls.lock().unwrap().push(duration);
        Box::pin(async {})
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

        let calls = sleeper.calls();
        assert_eq!(calls.len(), 3);
        assert_eq!(calls[0], Duration::from_millis(100));
        assert_eq!(calls[1], Duration::from_millis(200));
        assert_eq!(calls[2], Duration::from_millis(400));
    }

    #[tokio::test]
    async fn tracking_sleeper_can_clear() {
        let sleeper = TrackingSleeper::new();

        sleeper.sleep(Duration::from_millis(100)).await;
        assert_eq!(sleeper.calls().len(), 1);

        sleeper.clear();
        assert_eq!(sleeper.calls().len(), 0);

        sleeper.sleep(Duration::from_millis(200)).await;
        assert_eq!(sleeper.calls().len(), 1);
        assert_eq!(sleeper.calls()[0], Duration::from_millis(200));
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
