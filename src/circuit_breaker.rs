//! Circuit breaker implementation with lock-free atomics

use crate::ResilienceError;
use std::future::Future;
use std::sync::atomic::{AtomicU64, AtomicU8, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

const STATE_CLOSED: u8 = 0;
const STATE_OPEN: u8 = 1;
const STATE_HALF_OPEN: u8 = 2;

/// Clock abstraction so circuit breaker timing can be faked in tests
pub trait Clock: Send + Sync + std::fmt::Debug {
    fn now_millis(&self) -> u64;
}

/// Monotonic clock backed by `Instant::now()`
#[derive(Debug, Clone)]
pub struct MonotonicClock {
    start: Instant,
}

impl Default for MonotonicClock {
    fn default() -> Self {
        Self {
            start: Instant::now(),
        }
    }
}

impl Clock for MonotonicClock {
    fn now_millis(&self) -> u64 {
        self.start.elapsed().as_millis() as u64
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

#[derive(Clone)]
pub struct CircuitBreakerConfig {
    pub failure_threshold: usize,
    pub recovery_timeout: Duration,
    pub half_open_max_calls: usize,
}

impl CircuitBreakerConfig {
    pub fn disabled() -> Self {
        Self {
            failure_threshold: usize::MAX,
            recovery_timeout: Duration::from_secs(0),
            half_open_max_calls: usize::MAX,
        }
    }
}

struct CircuitBreakerState {
    state: AtomicU8,
    failure_count: AtomicUsize,
    opened_at_millis: AtomicU64,
    half_open_calls: AtomicUsize,
}

#[derive(Clone)]
pub struct CircuitBreakerPolicy {
    state: Arc<CircuitBreakerState>,
    config: CircuitBreakerConfig,
    clock: Arc<dyn Clock>,
}

impl CircuitBreakerPolicy {
    pub fn new(failure_threshold: usize, recovery_timeout: Duration) -> Self {
        Self {
            state: Arc::new(CircuitBreakerState {
                state: AtomicU8::new(STATE_CLOSED),
                failure_count: AtomicUsize::new(0),
                opened_at_millis: AtomicU64::new(0),
                half_open_calls: AtomicUsize::new(0),
            }),
            config: CircuitBreakerConfig {
                failure_threshold,
                recovery_timeout,
                half_open_max_calls: 1,
            },
            clock: Arc::new(MonotonicClock::default()),
        }
    }

    pub fn with_config(config: CircuitBreakerConfig) -> Self {
        Self {
            state: Arc::new(CircuitBreakerState {
                state: AtomicU8::new(STATE_CLOSED),
                failure_count: AtomicUsize::new(0),
                opened_at_millis: AtomicU64::new(0),
                half_open_calls: AtomicUsize::new(0),
            }),
            config,
            clock: Arc::new(MonotonicClock::default()),
        }
    }

    /// Override the clock (useful for deterministic tests)
    pub fn with_clock<C: Clock + 'static>(mut self, clock: C) -> Self {
        self.clock = Arc::new(clock);
        self
    }

    pub fn with_half_open_limit(mut self, limit: usize) -> Self {
        self.config.half_open_max_calls = limit;
        self
    }

    pub async fn execute<T, E, Fut, Op>(&self, mut operation: Op) -> Result<T, ResilienceError<E>>
    where
        T: Send,
        E: std::error::Error + Send + Sync + 'static,
        Fut: Future<Output = Result<T, ResilienceError<E>>> + Send,
        Op: FnMut() -> Fut + Send,
    {
        // Check state and enforce policy
        loop {
            let current_state = self.state.state.load(Ordering::Acquire);

            match current_state {
                STATE_OPEN => {
                    let opened_at = self.state.opened_at_millis.load(Ordering::Acquire);
                    let now = self.now_millis();
                    let elapsed = now.saturating_sub(opened_at);

                    if elapsed >= self.config.recovery_timeout.as_millis() as u64 {
                        // Try transition to half-open
                        match self.state.state.compare_exchange(
                            STATE_OPEN,
                            STATE_HALF_OPEN,
                            Ordering::AcqRel,
                            Ordering::Acquire,
                        ) {
                            Ok(_) => {
                                // We won the race - we're the first half-open caller
                                tracing::info!("Circuit breaker → half-open");
                                self.state.half_open_calls.store(1, Ordering::Release);
                                break; // Proceed to execute
                            }
                            Err(STATE_HALF_OPEN) => {
                                // Someone else transitioned to half-open
                                // Re-check on next iteration
                                continue;
                            }
                            Err(STATE_CLOSED) => {
                                // Someone else closed it - we're good
                                break;
                            }
                            Err(_) => unreachable!("Invalid state transition"),
                        }
                    } else {
                        // Still in timeout period
                        return Err(ResilienceError::CircuitOpen {
                            failure_count: self.state.failure_count.load(Ordering::Acquire),
                            open_duration: Duration::from_millis(elapsed),
                        });
                    }
                }
                STATE_HALF_OPEN => {
                    // Limit concurrent test requests
                    let current = self.state.half_open_calls.fetch_add(1, Ordering::AcqRel);
                    if current >= self.config.half_open_max_calls {
                        self.state.half_open_calls.fetch_sub(1, Ordering::Release);
                        return Err(ResilienceError::CircuitOpen {
                            failure_count: self.state.failure_count.load(Ordering::Acquire),
                            open_duration: Duration::from_millis(0),
                        });
                    }
                    tracing::debug!(
                        in_flight = current + 1,
                        max = self.config.half_open_max_calls,
                        "Circuit breaker: half-open test request"
                    );
                    break; // Proceed to execute
                }
                STATE_CLOSED => {
                    break; // Normal operation
                }
                _ => unreachable!("Invalid circuit breaker state"),
            }
        }

        // Execute the operation
        let was_half_open = self.state.state.load(Ordering::Acquire) == STATE_HALF_OPEN;
        let result = operation().await;

        // Decrement half-open counter if needed
        if was_half_open {
            self.state.half_open_calls.fetch_sub(1, Ordering::Release);
        }

        // Update state based on result
        match &result {
            Ok(_) => self.on_success(),
            Err(_) => self.on_failure(),
        }

        result
    }

    fn on_success(&self) {
        let current = self.state.state.load(Ordering::Acquire);

        match current {
            STATE_HALF_OPEN => {
                if self
                    .state
                    .state
                    .compare_exchange(
                        STATE_HALF_OPEN,
                        STATE_CLOSED,
                        Ordering::AcqRel,
                        Ordering::Acquire,
                    )
                    .is_ok()
                {
                    self.state.failure_count.store(0, Ordering::Release);
                    self.state.opened_at_millis.store(0, Ordering::Release);
                    tracing::info!("Circuit breaker → closed");
                }
            }
            STATE_CLOSED => {
                self.state.failure_count.store(0, Ordering::Release);
            }
            _ => {}
        }
    }

    fn on_failure(&self) {
        let current = self.state.state.load(Ordering::Acquire);
        let failures = self.state.failure_count.fetch_add(1, Ordering::AcqRel) + 1;

        match current {
            STATE_HALF_OPEN => {
                if self
                    .state
                    .state
                    .compare_exchange(
                        STATE_HALF_OPEN,
                        STATE_OPEN,
                        Ordering::AcqRel,
                        Ordering::Acquire,
                    )
                    .is_ok()
                {
                    self.state
                        .opened_at_millis
                        .store(self.now_millis(), Ordering::Release);
                    tracing::warn!(failures, "Circuit breaker: test failed → open");
                }
            }
            STATE_CLOSED => {
                if failures >= self.config.failure_threshold {
                    if self
                        .state
                        .state
                        .compare_exchange(
                            STATE_CLOSED,
                            STATE_OPEN,
                            Ordering::AcqRel,
                            Ordering::Acquire,
                        )
                        .is_ok()
                    {
                        self.state
                            .opened_at_millis
                            .store(self.now_millis(), Ordering::Release);
                        tracing::error!(
                            failures,
                            threshold = self.config.failure_threshold,
                            "Circuit breaker → open"
                        );
                    }
                }
            }
            _ => {}
        }
    }

    fn now_millis(&self) -> u64 {
        self.clock.now_millis()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
    use std::sync::Arc;

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct TestError(String);

    impl std::fmt::Display for TestError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "TestError: {}", self.0)
        }
    }

    impl std::error::Error for TestError {}

    #[derive(Debug, Clone)]
    struct ManualClock {
        now: Arc<AtomicU64>,
    }

    impl ManualClock {
        fn new() -> Self {
            Self {
                now: Arc::new(AtomicU64::new(0)),
            }
        }

        fn advance(&self, millis: u64) {
            self.now.fetch_add(millis, Ordering::SeqCst);
        }
    }

    impl Clock for ManualClock {
        fn now_millis(&self) -> u64 {
            self.now.load(Ordering::SeqCst)
        }
    }

    #[tokio::test]
    async fn test_circuit_starts_closed() {
        let breaker = CircuitBreakerPolicy::new(3, Duration::from_secs(1));
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let result = breaker
            .execute(|| {
                let counter = counter_clone.clone();
                async move {
                    counter.fetch_add(1, Ordering::SeqCst);
                    Ok::<_, ResilienceError<TestError>>(42)
                }
            })
            .await;

        assert_eq!(result.unwrap(), 42);
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_circuit_opens_after_threshold_failures() {
        let breaker = CircuitBreakerPolicy::new(3, Duration::from_secs(10));
        let counter = Arc::new(AtomicUsize::new(0));

        // Trigger 3 failures to open the circuit
        for _ in 0..3 {
            let counter_clone = counter.clone();
            let _ = breaker
                .execute(|| {
                    let counter = counter_clone.clone();
                    async move {
                        counter.fetch_add(1, Ordering::SeqCst);
                        Err::<(), _>(ResilienceError::Inner(TestError("fail".to_string())))
                    }
                })
                .await;
        }

        assert_eq!(
            counter.load(Ordering::SeqCst),
            3,
            "Should have executed 3 times"
        );

        // Next call should fail immediately without executing
        counter.store(0, Ordering::SeqCst);
        let counter_clone = counter.clone();
        let result = breaker
            .execute(|| {
                let counter = counter_clone.clone();
                async move {
                    counter.fetch_add(1, Ordering::SeqCst);
                    Ok::<_, ResilienceError<TestError>>(42)
                }
            })
            .await;

        assert!(result.is_err());
        assert!(result.unwrap_err().is_circuit_open());
        assert_eq!(
            counter.load(Ordering::SeqCst),
            0,
            "Should not execute when circuit is open"
        );
    }

    #[tokio::test]
    async fn test_circuit_transitions_to_half_open_after_timeout() {
        let breaker = CircuitBreakerPolicy::new(2, Duration::from_millis(100));
        let counter = Arc::new(AtomicUsize::new(0));

        // Open the circuit with 2 failures
        for _ in 0..2 {
            let counter_clone = counter.clone();
            let _ = breaker
                .execute(|| {
                    let counter = counter_clone.clone();
                    async move {
                        counter.fetch_add(1, Ordering::SeqCst);
                        Err::<(), _>(ResilienceError::Inner(TestError("fail".to_string())))
                    }
                })
                .await;
        }

        // Verify circuit is open
        counter.store(0, Ordering::SeqCst);
        let counter_clone = counter.clone();
        let result = breaker
            .execute(|| {
                let counter = counter_clone.clone();
                async move {
                    counter.fetch_add(1, Ordering::SeqCst);
                    Ok::<_, ResilienceError<TestError>>(42)
                }
            })
            .await;
        assert!(result.unwrap_err().is_circuit_open());
        assert_eq!(counter.load(Ordering::SeqCst), 0);

        // Wait for recovery timeout
        tokio::time::sleep(Duration::from_millis(150)).await;

        // Should now allow test call (half-open)
        counter.store(0, Ordering::SeqCst);
        let counter_clone = counter.clone();
        let result = breaker
            .execute(|| {
                let counter = counter_clone.clone();
                async move {
                    counter.fetch_add(1, Ordering::SeqCst);
                    Ok::<_, ResilienceError<TestError>>(100)
                }
            })
            .await;

        assert_eq!(result.unwrap(), 100);
        assert_eq!(
            counter.load(Ordering::SeqCst),
            1,
            "Should execute in half-open state"
        );
    }

    #[tokio::test]
    async fn test_circuit_closes_after_successful_half_open_test() {
        let breaker = CircuitBreakerPolicy::new(2, Duration::from_millis(100));
        let counter = Arc::new(AtomicUsize::new(0));

        // Open the circuit
        for _ in 0..2 {
            let counter_clone = counter.clone();
            let _ = breaker
                .execute(|| {
                    let counter = counter_clone.clone();
                    async move {
                        counter.fetch_add(1, Ordering::SeqCst);
                        Err::<(), _>(ResilienceError::Inner(TestError("fail".to_string())))
                    }
                })
                .await;
        }

        // Wait and succeed in half-open
        tokio::time::sleep(Duration::from_millis(150)).await;
        let counter_clone = counter.clone();
        let _ = breaker
            .execute(|| {
                let counter = counter_clone.clone();
                async move {
                    counter.fetch_add(1, Ordering::SeqCst);
                    Ok::<_, ResilienceError<TestError>>(42)
                }
            })
            .await;

        // Circuit should now be closed - multiple calls should succeed
        counter.store(0, Ordering::SeqCst);
        for _ in 0..5 {
            let counter_clone = counter.clone();
            let result = breaker
                .execute(|| {
                    let counter = counter_clone.clone();
                    async move {
                        counter.fetch_add(1, Ordering::SeqCst);
                        Ok::<_, ResilienceError<TestError>>(42)
                    }
                })
                .await;
            assert!(result.is_ok());
        }
        assert_eq!(
            counter.load(Ordering::SeqCst),
            5,
            "All calls should succeed when closed"
        );
    }

    #[tokio::test]
    async fn test_circuit_reopens_if_half_open_test_fails() {
        let breaker = CircuitBreakerPolicy::new(2, Duration::from_millis(100));

        // Open the circuit
        for _ in 0..2 {
            let _ = breaker
                .execute(|| async {
                    Err::<(), _>(ResilienceError::Inner(TestError("fail".to_string())))
                })
                .await;
        }

        // Wait and fail in half-open
        tokio::time::sleep(Duration::from_millis(150)).await;
        let _ = breaker
            .execute(|| async {
                Err::<(), _>(ResilienceError::Inner(TestError("fail again".to_string())))
            })
            .await;

        // Circuit should be open again
        let result = breaker
            .execute(|| async { Ok::<_, ResilienceError<TestError>>(42) })
            .await;

        assert!(result.is_err());
        assert!(result.unwrap_err().is_circuit_open());
    }

    #[tokio::test]
    async fn test_half_open_limits_concurrent_calls() {
        let breaker =
            CircuitBreakerPolicy::new(2, Duration::from_millis(100)).with_half_open_limit(1);
        let counter = Arc::new(AtomicUsize::new(0));

        // Open the circuit
        for _ in 0..2 {
            let _ = breaker
                .execute(|| async {
                    Err::<(), _>(ResilienceError::Inner(TestError("fail".to_string())))
                })
                .await;
        }

        // Wait for recovery
        tokio::time::sleep(Duration::from_millis(150)).await;

        // Launch 3 concurrent calls - only 1 should be allowed through
        let mut handles = vec![];
        for _ in 0..3 {
            let breaker_clone = breaker.clone();
            let counter_clone = counter.clone();
            let handle = tokio::spawn(async move {
                breaker_clone
                    .execute(|| {
                        let counter = counter_clone.clone();
                        async move {
                            counter.fetch_add(1, Ordering::SeqCst);
                            tokio::time::sleep(Duration::from_millis(50)).await;
                            Ok::<_, ResilienceError<TestError>>(42)
                        }
                    })
                    .await
            });
            handles.push(handle);
        }

        let results: Vec<_> = futures::future::join_all(handles).await;

        let successes = results
            .iter()
            .filter(|r| r.as_ref().unwrap().is_ok())
            .count();
        let circuit_opens = results
            .iter()
            .filter(|r| {
                r.as_ref()
                    .unwrap()
                    .as_ref()
                    .err()
                    .map_or(false, |e| e.is_circuit_open())
            })
            .count();

        assert_eq!(successes, 1, "Only 1 call should succeed in half-open");
        assert_eq!(circuit_opens, 2, "Other 2 calls should be rejected");
    }

    #[tokio::test]
    async fn test_disabled_circuit_breaker_never_opens() {
        let breaker = CircuitBreakerPolicy::with_config(CircuitBreakerConfig::disabled());
        let counter = Arc::new(AtomicUsize::new(0));

        // Trigger many failures
        for _ in 0..1000 {
            let counter_clone = counter.clone();
            let _ = breaker
                .execute(|| {
                    let counter = counter_clone.clone();
                    async move {
                        counter.fetch_add(1, Ordering::SeqCst);
                        Err::<(), _>(ResilienceError::Inner(TestError("fail".to_string())))
                    }
                })
                .await;
        }

        assert_eq!(
            counter.load(Ordering::SeqCst),
            1000,
            "All calls should execute with disabled breaker"
        );

        // One more call should still work
        counter.store(0, Ordering::SeqCst);
        let counter_clone = counter.clone();
        let result = breaker
            .execute(|| {
                let counter = counter_clone.clone();
                async move {
                    counter.fetch_add(1, Ordering::SeqCst);
                    Ok::<_, ResilienceError<TestError>>(42)
                }
            })
            .await;

        assert_eq!(result.unwrap(), 42);
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_successes_in_closed_state_reset_failure_count() {
        let breaker = CircuitBreakerPolicy::new(3, Duration::from_secs(1));

        // 2 failures (not enough to open)
        for _ in 0..2 {
            let _ = breaker
                .execute(|| async {
                    Err::<(), _>(ResilienceError::Inner(TestError("fail".to_string())))
                })
                .await;
        }

        // 1 success (should reset count)
        let _ = breaker
            .execute(|| async { Ok::<_, ResilienceError<TestError>>(42) })
            .await;

        // 2 more failures (should not open since count was reset)
        for _ in 0..2 {
            let result = breaker
                .execute(|| async {
                    Err::<(), _>(ResilienceError::Inner(TestError("fail".to_string())))
                })
                .await;
            // Should still execute, not be circuit-open
            assert!(result.is_err());
            if let Err(ResilienceError::Inner(_)) = result {
                // This is correct - the operation failed, not the circuit
            } else {
                panic!("Expected Inner error, not circuit open");
            }
        }
    }

    #[tokio::test]
    async fn test_custom_clock_allows_instant_recovery() {
        let clock = ManualClock::new();
        let breaker =
            CircuitBreakerPolicy::new(1, Duration::from_millis(100)).with_clock(clock.clone());

        // First call fails → opens circuit
        let _ = breaker
            .execute(|| async {
                Err::<(), _>(ResilienceError::Inner(TestError("fail".to_string())))
            })
            .await;

        // Immediately try again: should still be open (0ms elapsed)
        let open_result = breaker
            .execute(|| async { Ok::<_, ResilienceError<TestError>>(()) })
            .await;
        assert!(open_result.unwrap_err().is_circuit_open());

        // Advance virtual clock beyond recovery timeout
        clock.advance(150);

        // Should transition to half-open and allow a successful call
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();
        let success = breaker
            .execute(|| {
                let counter = counter_clone.clone();
                async move {
                    counter.fetch_add(1, Ordering::SeqCst);
                    Ok::<_, ResilienceError<TestError>>(42)
                }
            })
            .await;

        assert_eq!(success.unwrap(), 42);
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }
}
