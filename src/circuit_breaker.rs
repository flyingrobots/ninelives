//! Circuit breaker implementation with lock-free atomics

use crate::{clock::Clock, clock::MonotonicClock, ResilienceError};
use std::future::Future;
use std::sync::atomic::{AtomicU64, AtomicU8, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

const STATE_CLOSED: u8 = 0;
const STATE_OPEN: u8 = 1;
const STATE_HALF_OPEN: u8 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CircuitStateConversionError {
    InvalidValue(u8),
}

/// Current state of the circuit breaker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    /// Normal operating mode.
    Closed,
    /// Short-circuits calls until recovery timeout elapses.
    Open,
    /// Probe mode allowing a limited number of calls to test recovery.
    HalfOpen,
}

impl CircuitState {
    fn to_u8(self) -> u8 {
        match self {
            CircuitState::Closed => STATE_CLOSED,
            CircuitState::Open => STATE_OPEN,
            CircuitState::HalfOpen => STATE_HALF_OPEN,
        }
    }
}

fn u8_to_state(v: u8) -> Result<CircuitState, CircuitStateConversionError> {
    match v {
        STATE_CLOSED => Ok(CircuitState::Closed),
        STATE_OPEN => Ok(CircuitState::Open),
        STATE_HALF_OPEN => Ok(CircuitState::HalfOpen),
        other => Err(CircuitStateConversionError::InvalidValue(other)),
    }
}

/// Validated configuration for the circuit breaker.
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    failure_threshold: usize,
    recovery_timeout: Duration,
    half_open_max_calls: usize,
}

/// Errors produced when validating breaker configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CircuitBreakerError {
    /// Failure threshold must be > 0.
    InvalidFailureThreshold {
        /// Value provided by caller.
        provided: usize,
    },
    /// Recovery timeout must be > 0 unless breaker disabled.
    InvalidRecoveryTimeout(Duration),
    /// Half-open probe limit must be > 0.
    InvalidHalfOpenLimit {
        /// Value provided by caller.
        provided: usize,
    },
}

impl std::fmt::Display for CircuitBreakerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CircuitBreakerError::InvalidFailureThreshold { provided } => {
                write!(f, "failure_threshold must be > 0 (got {})", provided)
            }
            CircuitBreakerError::InvalidRecoveryTimeout(timeout) => write!(
                f,
                "recovery_timeout must be > 0 unless breaker is disabled (got {:?})",
                timeout
            ),
            CircuitBreakerError::InvalidHalfOpenLimit { provided } => {
                write!(f, "half_open_max_calls must be > 0 (got {})", provided)
            }
        }
    }
}

impl std::error::Error for CircuitBreakerError {}

impl CircuitBreakerConfig {
    /// Create a config with validation.
    pub fn new(
        failure_threshold: usize,
        recovery_timeout: Duration,
        half_open_max_calls: usize,
    ) -> Result<Self, CircuitBreakerError> {
        let cfg = Self { failure_threshold, recovery_timeout, half_open_max_calls };
        CircuitBreakerPolicy::validate_config(&cfg)?;
        Ok(cfg)
    }

    /// Creates a disabled circuit breaker that never opens.
    /// Uses `usize::MAX` thresholds and `Duration::MAX` timeout to effectively disable all circuit breaking logic.
    pub fn disabled() -> Self {
        Self {
            failure_threshold: usize::MAX,
            recovery_timeout: Duration::MAX,
            half_open_max_calls: usize::MAX,
        }
    }

    /// Threshold before opening from Closed.
    pub fn failure_threshold(&self) -> usize {
        self.failure_threshold
    }

    /// Duration to stay Open before Half-Open probes.
    pub fn recovery_timeout(&self) -> Duration {
        self.recovery_timeout
    }

    /// Maximum concurrent calls while Half-Open.
    pub fn half_open_max_calls(&self) -> usize {
        self.half_open_max_calls
    }
}

#[derive(Debug)]
struct CircuitBreakerState {
    state: AtomicU8,
    failure_count: AtomicUsize,
    opened_at_millis: AtomicU64,
    half_open_calls: AtomicUsize,
}

#[derive(Debug, Clone)]
/// Circuit breaker policy guarding an async operation.
/// Clones share the same underlying state via `Arc`, so all handles observe and affect the same
/// circuit lifecycle (failure counts, open/half-open/closed transitions).
pub struct CircuitBreakerPolicy {
    state: Arc<CircuitBreakerState>,
    config: CircuitBreakerConfig,
    clock: Arc<dyn Clock>,
}

impl CircuitBreakerPolicy {
    /// Create a circuit breaker policy, validating thresholds and timeouts.
    /// Errors if `failure_threshold` == 0, `recovery_timeout` == 0 for enabled breakers, or
    /// `half_open_max_calls` == 0. Defaults `half_open_max_calls` to 1.
    ///
    /// # Examples
    /// ```
    /// use ninelives::CircuitBreakerPolicy;
    /// use std::time::Duration;
    /// let breaker = CircuitBreakerPolicy::new(5, Duration::from_secs(30)).unwrap();
    /// ```
    pub fn new(
        failure_threshold: usize,
        recovery_timeout: Duration,
    ) -> Result<Self, CircuitBreakerError> {
        let config =
            CircuitBreakerConfig { failure_threshold, recovery_timeout, half_open_max_calls: 1 };

        Self::validate_config(&config)?;
        Ok(Self::from_config(config))
    }

    /// Create a breaker from an explicit config, validating the values.
    /// Use [`CircuitBreakerConfig::new`] to build a validated config.
    pub fn with_config(config: CircuitBreakerConfig) -> Result<Self, CircuitBreakerError> {
        Self::validate_config(&config)?;
        Ok(Self::from_config(config))
    }

    /// Override the clock (useful for deterministic tests).
    ///
    /// # Example
    /// ```
    /// # use ninelives::CircuitBreakerPolicy;
    /// # use std::time::Duration;
    /// #[derive(Debug)]
    /// struct NoopClock;
    /// impl ninelives::Clock for NoopClock { fn now_millis(&self) -> u64 { 0 } }
    /// let breaker = CircuitBreakerPolicy::new(1, Duration::from_secs(1))
    ///     .unwrap()
    ///     .with_clock(NoopClock);
    /// ```
    pub fn with_clock<C: Clock + 'static>(mut self, clock: C) -> Self {
        self.clock = Arc::new(clock);
        self
    }

    /// Override the maximum number of half-open probe calls; must be > 0.
    pub fn with_half_open_limit(mut self, limit: usize) -> Result<Self, CircuitBreakerError> {
        if limit == 0 {
            return Err(CircuitBreakerError::InvalidHalfOpenLimit { provided: limit });
        }
        self.config.half_open_max_calls = limit;
        Ok(self)
    }

    fn from_config(config: CircuitBreakerConfig) -> Self {
        Self { state: Self::new_state(), config, clock: Arc::new(MonotonicClock::default()) }
    }

    fn new_state() -> Arc<CircuitBreakerState> {
        Arc::new(CircuitBreakerState {
            state: AtomicU8::new(CircuitState::Closed.to_u8()),
            failure_count: AtomicUsize::new(0),
            opened_at_millis: AtomicU64::new(0),
            half_open_calls: AtomicUsize::new(0),
        })
    }

    fn validate_config(config: &CircuitBreakerConfig) -> Result<(), CircuitBreakerError> {
        if config.failure_threshold == 0 {
            return Err(CircuitBreakerError::InvalidFailureThreshold { provided: 0 });
        }

        if config.half_open_max_calls == 0 {
            return Err(CircuitBreakerError::InvalidHalfOpenLimit { provided: 0 });
        }

        let disabled = config.failure_threshold == usize::MAX;
        if config.recovery_timeout == Duration::ZERO && !disabled {
            return Err(CircuitBreakerError::InvalidRecoveryTimeout(config.recovery_timeout));
        }

        Ok(())
    }

    /// Executes the provided async operation under circuit breaker protection.
    ///
    /// # Behavior
    /// - **Closed**: Executes the operation normally. Consecutive failures increment the failure count.
    /// - **Open**: Rejects calls with `ResilienceError::CircuitOpen` until `recovery_timeout` elapses.
    /// - **HalfOpen**: Allows limited test calls (`half_open_max_calls`). Success closes the circuit; failure reopens it.
    ///
    /// # Errors
    /// Returns `ResilienceError::CircuitOpen` if the circuit is open or half-open capacity is exceeded.
    /// Returns `ResilienceError::Inner(E)` if the operation itself fails.
    pub async fn execute<T, E, Fut, Op>(&self, operation: Op) -> Result<T, ResilienceError<E>>
    where
        T: Send,
        E: std::error::Error + Send + Sync + 'static,
        Fut: Future<Output = Result<T, ResilienceError<E>>> + Send,
        Op: FnOnce() -> Fut + Send,
    {
        // Check state and enforce policy
        struct HalfOpenGuard<'a> {
            state: &'a CircuitBreakerState,
            did_increment: bool,
        }
        impl<'a> Drop for HalfOpenGuard<'a> {
            fn drop(&mut self) {
                if self.did_increment {
                    self.state.half_open_calls.fetch_sub(1, Ordering::Release);
                }
            }
        }
        let mut guard: Option<HalfOpenGuard<'_>> = None;

        loop {
            let current_state_raw = self.state.state.load(Ordering::Acquire);
            let current_state = match u8_to_state(current_state_raw) {
                Ok(s) => s,
                Err(_) => {
                    return Err(ResilienceError::CircuitOpen {
                        failure_count: self.state.failure_count.load(Ordering::Acquire),
                        open_duration: Duration::from_millis(0),
                    })
                }
            };

            match current_state {
                CircuitState::Open => {
                    let opened_at = self.state.opened_at_millis.load(Ordering::Acquire);
                    let now = self.now_millis();
                    let elapsed = now.saturating_sub(opened_at);

                    if elapsed >= self.config.recovery_timeout.as_millis() as u64 {
                        // Try transition to half-open
                        match self.state.state.compare_exchange(
                            CircuitState::Open.to_u8(),
                            CircuitState::HalfOpen.to_u8(),
                            Ordering::AcqRel,
                            Ordering::Acquire,
                        ) {
                            Ok(_) => {
                                // We won the race - we're the first half-open caller
                                tracing::info!("Circuit breaker → half-open");
                                self.state.half_open_calls.store(1, Ordering::Release);
                                guard =
                                    Some(HalfOpenGuard { state: &self.state, did_increment: true });
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
                            Err(other) => unreachable!(
                                "compare_exchange returned unexpected state: {}",
                                other
                            ),
                        }
                    } else {
                        // Still in timeout period
                        return Err(ResilienceError::CircuitOpen {
                            failure_count: self.state.failure_count.load(Ordering::Acquire),
                            open_duration: Duration::from_millis(elapsed),
                        });
                    }
                }
                CircuitState::HalfOpen => {
                    // Limit concurrent test requests
                    let current = self.state.half_open_calls.fetch_add(1, Ordering::AcqRel);
                    if current >= self.config.half_open_max_calls {
                        self.state.half_open_calls.fetch_sub(1, Ordering::Release);
                        let opened_at = self.state.opened_at_millis.load(Ordering::Acquire);
                        let elapsed = self.now_millis().saturating_sub(opened_at);
                        return Err(ResilienceError::CircuitOpen {
                            failure_count: self.state.failure_count.load(Ordering::Acquire),
                            open_duration: Duration::from_millis(elapsed),
                        });
                    }
                    guard = Some(HalfOpenGuard { state: &self.state, did_increment: true });
                    tracing::debug!(
                        in_flight = current + 1,
                        max = self.config.half_open_max_calls,
                        "Circuit breaker: half-open test request"
                    );
                    break; // Proceed to execute
                }
                CircuitState::Closed => {
                    break; // Normal operation
                }
            }
        }

        // Execute the operation
        let result = operation().await;
        drop(guard);

        // Update state based on result
        match &result {
            Ok(_) => self.on_success(),
            Err(_) => self.on_failure(),
        }

        result
    }

    /// Resets consecutive failure count; any success in the closed state resets the counter to 0,
    /// meaning only consecutive failures trip the breaker (patterns like F-F-S-F-F will not open it
    /// unless the final streak meets `failure_threshold`).
    fn on_success(&self) {
        let current_raw = self.state.state.load(Ordering::Acquire);
        let current = u8_to_state(current_raw).unwrap_or(CircuitState::Closed);

        match current {
            CircuitState::HalfOpen => {
                if self
                    .state
                    .state
                    .compare_exchange(
                        CircuitState::HalfOpen.to_u8(),
                        CircuitState::Closed.to_u8(),
                        Ordering::AcqRel,
                        Ordering::Acquire,
                    )
                    .is_ok()
                {
                    self.state.half_open_calls.store(0, Ordering::Release);
                    self.state.failure_count.store(0, Ordering::Release);
                    self.state.opened_at_millis.store(0, Ordering::Release);
                    tracing::info!("Circuit breaker → closed");
                }
            }
            CircuitState::Closed => {
                self.state.failure_count.store(0, Ordering::Release);
            }
            _ => {}
        }
    }

    fn on_failure(&self) {
        let current_raw = self.state.state.load(Ordering::Acquire);
        let current = u8_to_state(current_raw).unwrap_or(CircuitState::Closed);
        let failures = self.state.failure_count.fetch_add(1, Ordering::AcqRel) + 1;

        match current {
            CircuitState::HalfOpen => {
                if self
                    .state
                    .state
                    .compare_exchange(
                        CircuitState::HalfOpen.to_u8(),
                        CircuitState::Open.to_u8(),
                        Ordering::AcqRel,
                        Ordering::Acquire,
                    )
                    .is_ok()
                {
                    self.state.half_open_calls.store(0, Ordering::Release);
                    self.state.opened_at_millis.store(self.now_millis(), Ordering::Release);
                    tracing::warn!(failures, "Circuit breaker: test failed → open");
                }
            }
            CircuitState::Closed => {
                if failures >= self.config.failure_threshold
                    && self
                        .state
                        .state
                        .compare_exchange(
                            CircuitState::Closed.to_u8(),
                            CircuitState::Open.to_u8(),
                            Ordering::AcqRel,
                            Ordering::Acquire,
                        )
                        .is_ok()
                {
                    self.state.half_open_calls.store(0, Ordering::Release);
                    self.state.opened_at_millis.store(self.now_millis(), Ordering::Release);
                    tracing::error!(
                        failures,
                        threshold = self.config.failure_threshold,
                        "Circuit breaker → open"
                    );
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
    use futures::future::join_all;
    use futures::FutureExt;
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
            Self { now: Arc::new(AtomicU64::new(0)) }
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

    #[test]
    fn rejects_zero_failure_threshold() {
        let err = CircuitBreakerPolicy::new(0, Duration::from_secs(1))
            .expect_err("zero failures should be invalid");
        assert!(matches!(err, CircuitBreakerError::InvalidFailureThreshold { provided: 0 }));
    }

    #[test]
    fn rejects_zero_timeout_when_enabled() {
        let err = CircuitBreakerPolicy::new(1, Duration::ZERO)
            .expect_err("zero timeout should be invalid for enabled breaker");
        assert!(matches!(err, CircuitBreakerError::InvalidRecoveryTimeout(Duration::ZERO)));
    }

    #[test]
    fn rejects_zero_half_open_limit() {
        let err = CircuitBreakerPolicy::new(1, Duration::from_secs(1))
            .and_then(|breaker| breaker.with_half_open_limit(0))
            .expect_err("zero half-open limit should be invalid");

        assert!(matches!(err, CircuitBreakerError::InvalidHalfOpenLimit { provided: 0 }));
    }

    #[tokio::test]
    async fn test_circuit_starts_closed() {
        let breaker =
            CircuitBreakerPolicy::new(3, Duration::from_secs(1)).expect("valid circuit breaker");
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
        let breaker =
            CircuitBreakerPolicy::new(3, Duration::from_secs(10)).expect("valid circuit breaker");
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

        assert_eq!(counter.load(Ordering::SeqCst), 3, "Should have executed 3 times");

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
        assert_eq!(counter.load(Ordering::SeqCst), 0, "Should not execute when circuit is open");
    }

    #[tokio::test]
    async fn test_circuit_transitions_to_half_open_after_timeout() {
        let breaker = CircuitBreakerPolicy::new(2, Duration::from_millis(100))
            .expect("valid circuit breaker");
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
        assert_eq!(counter.load(Ordering::SeqCst), 1, "Should execute in half-open state");
    }

    #[tokio::test]
    async fn test_circuit_closes_after_successful_half_open_test() {
        let breaker = CircuitBreakerPolicy::new(2, Duration::from_millis(100))
            .expect("valid circuit breaker");
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
        assert_eq!(counter.load(Ordering::SeqCst), 5, "All calls should succeed when closed");
    }

    #[tokio::test]
    async fn test_circuit_reopens_if_half_open_test_fails() {
        let breaker = CircuitBreakerPolicy::new(2, Duration::from_millis(100))
            .expect("valid circuit breaker");

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
        let result = breaker.execute(|| async { Ok::<_, ResilienceError<TestError>>(42) }).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().is_circuit_open());
    }

    #[tokio::test]
    async fn test_half_open_limits_concurrent_calls() {
        let breaker = CircuitBreakerPolicy::new(2, Duration::from_millis(100))
            .expect("valid circuit breaker")
            .with_half_open_limit(1)
            .expect("valid half-open limit");
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

        let successes = results.iter().filter(|r| r.as_ref().expect("join error").is_ok()).count();
        let circuit_opens = results
            .iter()
            .filter(|r| {
                r.as_ref().expect("join error").as_ref().err().is_some_and(|e| e.is_circuit_open())
            })
            .count();

        assert_eq!(successes, 1, "Only 1 call should succeed in half-open");
        assert_eq!(circuit_opens, 2, "Other 2 calls should be rejected");
    }

    #[tokio::test]
    async fn test_disabled_circuit_breaker_never_opens() {
        let breaker = CircuitBreakerPolicy::with_config(CircuitBreakerConfig::disabled())
            .expect("disabled config should be valid");
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
        let breaker =
            CircuitBreakerPolicy::new(3, Duration::from_secs(1)).expect("valid circuit breaker");

        // 2 failures (not enough to open)
        for _ in 0..2 {
            let _ = breaker
                .execute(|| async {
                    Err::<(), _>(ResilienceError::Inner(TestError("fail".to_string())))
                })
                .await;
        }

        // 1 success (should reset count)
        let _ = breaker.execute(|| async { Ok::<_, ResilienceError<TestError>>(42) }).await;

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
        let breaker = CircuitBreakerPolicy::new(1, Duration::from_millis(100))
            .expect("valid circuit breaker")
            .with_clock(clock.clone());

        // First call fails → opens circuit
        let _ = breaker
            .execute(|| async {
                Err::<(), _>(ResilienceError::Inner(TestError("fail".to_string())))
            })
            .await;

        // Immediately try again: should still be open (0ms elapsed)
        let open_result =
            breaker.execute(|| async { Ok::<_, ResilienceError<TestError>>(()) }).await;
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

    #[tokio::test]
    async fn half_open_counter_recovers_on_panic() {
        let breaker = CircuitBreakerPolicy::new(1, Duration::from_millis(10)).unwrap();

        let _ = breaker
            .execute(|| async { Err::<(), _>(ResilienceError::Inner(TestError("x".into()))) })
            .await;
        tokio::time::sleep(Duration::from_millis(20)).await;

        let result: Result<Result<(), ResilienceError<TestError>>, _> =
            std::panic::AssertUnwindSafe(async {
                breaker.execute(|| async { panic!("boom") }).await
            })
            .catch_unwind()
            .await;
        assert!(result.is_err());
        assert_eq!(breaker.state.half_open_calls.load(Ordering::Acquire), 0);
    }

    #[tokio::test]
    async fn stress_concurrent_half_open_transitions() {
        let breaker = CircuitBreakerPolicy::new(1, Duration::from_millis(5)).unwrap();
        let _ = breaker
            .execute(|| async { Err::<(), _>(ResilienceError::Inner(TestError("x".into()))) })
            .await;
        tokio::time::sleep(Duration::from_millis(10)).await;

        let tasks = 200;
        let barrier = Arc::new(tokio::sync::Barrier::new(tasks));
        let mut handles = vec![];
        for _ in 0..tasks {
            let b = breaker.clone();
            let g = barrier.clone();
            handles.push(tokio::spawn(async move {
                g.wait().await;
                let _ = b
                    .execute(|| async {
                        Err::<(), _>(ResilienceError::Inner(TestError("y".into())))
                    })
                    .await;
            }));
        }

        let _ = join_all(handles).await;
        let in_half_open = breaker.state.half_open_calls.load(Ordering::Acquire);
        assert!(in_half_open <= breaker.config.half_open_max_calls);
    }
}
