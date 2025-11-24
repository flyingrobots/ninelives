//! Retry policy implementation
//!
//! Retry policy for fallible async operations.
//!
//! Semantics:
//! - `max_attempts` counts total attempts (initial try + retries).
//! - Only `ResilienceError::Inner(E)` values are eligible for retry; other variants return
//!   immediately.
//! - `should_retry` predicate decides whether an `Inner` error is retryable.
//! - Backoff calculates delay per retry attempt; jitter randomizes the delay to avoid thundering
//!   herds.
//! - Sleeper controls how delays are applied (production uses `TokioSleeper`; tests can inject
//!   `InstantSleeper`/`TrackingSleeper`).
//!
//! Invariants:
//! - Attempts never exceed `max_attempts`.
//! - Non-`Inner` errors are propagated without retry.
//! - Backoff/Jitter are invoked exactly retries-1 times.
//!
//! Example
//! ```rust
//! use std::time::Duration;
//! use ninelives::{Backoff, Jitter, RetryPolicy, ResilienceError};
//!
//! #[derive(Debug)]
//! struct MyErr;
//! impl std::fmt::Display for MyErr { fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "oops") } }
//! impl std::error::Error for MyErr {}
//!
//! # tokio::runtime::Runtime::new().unwrap().block_on(async {
//! let policy = RetryPolicy::<MyErr>::builder()
//!     .max_attempts(3) // total attempts
//!     .backoff(Backoff::exponential(Duration::from_millis(100)))
//!     .with_jitter(Jitter::full())
//!     .should_retry(|_e| true)
//!     .build()
//!     .unwrap();
//! let result: Result<(), ResilienceError<MyErr>> =
//!     policy.execute(|| async { Err(ResilienceError::Inner(MyErr)) }).await;
//! assert!(result.is_err());
//! # });
//! ```

use crate::error::MAX_RETRY_FAILURES;
use crate::{Backoff, Jitter, ResilienceError, Sleeper, TokioSleeper};
use futures::future::BoxFuture;
use std::collections::VecDeque;
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;
use tower_layer::Layer;
use tower_service::Service;

/// Retry policy combining backoff, jitter, predicate, and sleeper.
#[derive(Clone)]
pub struct RetryPolicy<E> {
    max_attempts: usize,
    backoff: Backoff,
    jitter: Jitter,
    should_retry: Arc<dyn Fn(&E) -> bool + Send + Sync>,
    sleeper: Arc<dyn Sleeper>,
}

impl<E> std::fmt::Debug for RetryPolicy<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RetryPolicy")
            .field("max_attempts", &self.max_attempts)
            .field("backoff", &self.backoff)
            .field("jitter", &self.jitter)
            .field("sleeper", &"<sleeper>")
            .field("should_retry", &"<predicate>")
            .finish()
    }
}

impl<E> RetryPolicy<E>
where
    E: std::error::Error + Send + Sync + 'static,
{
    /// Construct a new builder with defaults.
    pub fn builder() -> RetryPolicyBuilder<E> {
        RetryPolicyBuilder::new()
    }

    /// Execute an async operation with retry semantics.
    pub async fn execute<T, Fut, Op>(&self, mut operation: Op) -> Result<T, ResilienceError<E>>
    where
        T: Send,
        Fut: Future<Output = Result<T, ResilienceError<E>>> + Send,
        Op: FnMut() -> Fut + Send,
    {
        let mut failures: VecDeque<E> = VecDeque::new();

        for attempt in 0..self.max_attempts {
            match operation().await {
                Ok(value) => return Ok(value),
                Err(ResilienceError::Inner(e)) => {
                    // Check if we should retry this error
                    if !(self.should_retry)(&e) {
                        return Err(ResilienceError::Inner(e));
                    }

                    failures.push_back(e);
                    while failures.len() > MAX_RETRY_FAILURES {
                        failures.pop_front();
                    }

                    // If this was the last attempt, return RetryExhausted
                    if attempt + 1 >= self.max_attempts {
                        return Err(ResilienceError::retry_exhausted(
                            self.max_attempts,
                            failures.into_iter().collect(),
                        ));
                    }

                    // Calculate backoff delay for this retry (1-indexed: first retry uses delay(1))
                    let mut delay = self.backoff.delay(attempt + 1);

                    delay = match &self.jitter {
                        Jitter::Decorrelated(_) => self.jitter.apply_stateful(),
                        _ => self.jitter.apply(delay),
                    };

                    // Sleep before next attempt
                    self.sleeper.sleep(delay).await;
                }
                // Non-Inner errors (Timeout, Bulkhead, CircuitOpen) are not retried
                Err(e) => return Err(e),
            }
        }

        // Safety: unreachable because loop executes max_attempts times and each iteration
        // either returns or continues. On last iteration (attempt == max_attempts - 1),
        // we always return RetryExhausted for retryable errors.
        debug_assert!(false, "Retry loop should have returned; this indicates a logic bug");
        unreachable!()
    }
}

/// Builder for `RetryPolicy`.
pub struct RetryPolicyBuilder<E> {
    max_attempts: usize,
    backoff: Backoff,
    jitter: Jitter,
    should_retry: Arc<dyn Fn(&E) -> bool + Send + Sync>,
    sleeper: Arc<dyn Sleeper>,
}

/// Errors produced while building a retry policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuildError {
    /// `max_attempts` must be > 0.
    InvalidMaxAttempts(usize),
}

impl std::fmt::Display for BuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BuildError::InvalidMaxAttempts(n) => {
                write!(f, "max_attempts must be > 0 (got {})", n)
            }
        }
    }
}

impl std::error::Error for BuildError {}

impl<E> RetryPolicyBuilder<E>
where
    E: std::error::Error + Send + Sync + 'static,
{
    /// Create a builder with sane defaults.
    pub fn new() -> Self {
        Self {
            max_attempts: 3,
            backoff: Backoff::exponential(Duration::from_secs(1)).into(),
            jitter: Jitter::full(),
            should_retry: Arc::new(|_| true),
            sleeper: Arc::new(TokioSleeper),
        }
    }

    /// Set total attempts (initial + retries). Must be > 0.
    pub fn max_attempts(mut self, attempts: usize) -> Self {
        self.max_attempts = attempts;
        self
    }

    /// Set backoff strategy.
    pub fn backoff<B>(mut self, backoff: B) -> Self
    where
        B: Into<Backoff>,
    {
        self.backoff = backoff.into();
        self
    }

    /// Set jitter strategy.
    pub fn with_jitter(mut self, jitter: Jitter) -> Self {
        self.jitter = jitter;
        self
    }

    /// Predicate to decide if an `Inner` error is retryable.
    pub fn should_retry<F>(mut self, predicate: F) -> Self
    where
        F: Fn(&E) -> bool + Send + Sync + 'static,
    {
        self.should_retry = Arc::new(predicate);
        self
    }

    /// Provide a custom sleeper implementation.
    pub fn with_sleeper<S>(mut self, sleeper: S) -> Self
    where
        S: Sleeper + 'static,
    {
        self.sleeper = Arc::new(sleeper);
        self
    }

    /// Build the retry policy, validating inputs.
    pub fn build(self) -> Result<RetryPolicy<E>, BuildError> {
        if self.max_attempts == 0 {
            return Err(BuildError::InvalidMaxAttempts(0));
        }
        Ok(RetryPolicy {
            max_attempts: self.max_attempts,
            backoff: self.backoff,
            jitter: self.jitter,
            should_retry: self.should_retry,
            sleeper: self.sleeper,
        })
    }
}

impl<E> Default for RetryPolicyBuilder<E>
where
    E: std::error::Error + Send + Sync + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{InstantSleeper, TrackingSleeper};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct TestError(String);

    impl std::fmt::Display for TestError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "TestError: {}", self.0)
        }
    }

    impl std::error::Error for TestError {}

    #[tokio::test]
    async fn test_success_first_attempt() {
        let policy = RetryPolicy::builder()
            .max_attempts(3)
            .backoff(Backoff::constant(Duration::from_millis(100)))
            .with_sleeper(InstantSleeper)
            .build()
            .expect("builder");

        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let result = policy
            .execute(|| {
                let counter = counter_clone.clone();
                async move {
                    counter.fetch_add(1, Ordering::SeqCst);
                    Ok::<_, ResilienceError<TestError>>(42)
                }
            })
            .await;

        assert_eq!(result.unwrap(), 42);
        assert_eq!(counter.load(Ordering::SeqCst), 1, "Should only execute once");
    }

    #[tokio::test]
    async fn test_success_after_retries() {
        let policy = RetryPolicy::builder()
            .max_attempts(5)
            .backoff(Backoff::constant(Duration::from_millis(10)))
            .with_sleeper(InstantSleeper)
            .build()
            .expect("builder");

        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let result = policy
            .execute(|| {
                let counter = counter_clone.clone();
                async move {
                    let attempt = counter.fetch_add(1, Ordering::SeqCst);
                    if attempt < 2 {
                        Err(ResilienceError::Inner(TestError(format!("attempt {}", attempt))))
                    } else {
                        Ok(42)
                    }
                }
            })
            .await;

        assert_eq!(result.unwrap(), 42);
        assert_eq!(counter.load(Ordering::SeqCst), 3, "Should succeed on 3rd attempt");
    }

    #[tokio::test]
    async fn test_retry_exhaustion() {
        let policy = RetryPolicy::builder()
            .max_attempts(3)
            .backoff(Backoff::constant(Duration::from_millis(10)))
            .with_sleeper(InstantSleeper)
            .build()
            .expect("builder");

        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let result = policy
            .execute(|| {
                let counter = counter_clone.clone();
                async move {
                    let attempt = counter.fetch_add(1, Ordering::SeqCst);
                    Err::<(), _>(ResilienceError::Inner(TestError(format!("attempt {}", attempt))))
                }
            })
            .await;

        assert!(result.is_err());
        assert_eq!(counter.load(Ordering::SeqCst), 3, "Should attempt 3 times");

        match result.unwrap_err() {
            ResilienceError::RetryExhausted { attempts, failures } => {
                assert_eq!(attempts, 3);
                assert_eq!(failures.len(), 3);
                assert_eq!(failures[0].0, "attempt 0");
                assert_eq!(failures[1].0, "attempt 1");
                assert_eq!(failures[2].0, "attempt 2");
            }
            e => panic!("Expected RetryExhausted, got {:?}", e),
        }
    }

    #[tokio::test]
    async fn retry_exhausted_caps_stored_failures() {
        let policy = RetryPolicy::builder()
            .max_attempts(20)
            .backoff(Backoff::constant(Duration::from_millis(1)))
            .with_sleeper(InstantSleeper)
            .build()
            .expect("builder");

        let result = policy
            .execute(|| async {
                Err::<(), _>(ResilienceError::Inner(TestError("fail".to_string())))
            })
            .await;

        match result.unwrap_err() {
            ResilienceError::RetryExhausted { failures, .. } => {
                assert!(failures.len() <= crate::error::MAX_RETRY_FAILURES);
            }
            _ => panic!("expected retry exhausted"),
        }
    }

    #[tokio::test]
    async fn test_backoff_applied() {
        let sleeper = TrackingSleeper::new();
        let policy = RetryPolicy::builder()
            .max_attempts(4)
            .backoff(Backoff::linear(Duration::from_millis(100)))
            .with_jitter(Jitter::None)
            .with_sleeper(sleeper.clone())
            .build()
            .expect("builder");

        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let _ = policy
            .execute(|| {
                let counter = counter_clone.clone();
                async move {
                    counter.fetch_add(1, Ordering::SeqCst);
                    Err::<(), _>(ResilienceError::Inner(TestError("always fail".to_string())))
                }
            })
            .await;

        assert_eq!(sleeper.calls(), 3, "Should sleep 3 times (between 4 attempts)");

        // Linear backoff: 100ms, 200ms, 300ms
        assert_eq!(sleeper.call_at(0).unwrap(), Duration::from_millis(100));
        assert_eq!(sleeper.call_at(1).unwrap(), Duration::from_millis(200));
        assert_eq!(sleeper.call_at(2).unwrap(), Duration::from_millis(300));
    }

    #[tokio::test]
    async fn test_jitter_applied() {
        let sleeper = TrackingSleeper::new();
        let policy = RetryPolicy::builder()
            .max_attempts(3)
            .backoff(Backoff::constant(Duration::from_millis(100)))
            .with_jitter(Jitter::full())
            .with_sleeper(sleeper.clone())
            .build()
            .expect("builder");

        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let _ = policy
            .execute(|| {
                let counter = counter_clone.clone();
                async move {
                    counter.fetch_add(1, Ordering::SeqCst);
                    Err::<(), _>(ResilienceError::Inner(TestError("always fail".to_string())))
                }
            })
            .await;

        assert_eq!(sleeper.calls(), 2, "Should sleep 2 times (between 3 attempts)");

        // With full jitter, delays should be in range [0, 100ms]
        for idx in 0..sleeper.calls() {
            let call = sleeper.call_at(idx).unwrap();
            assert!(call <= Duration::from_millis(100), "Jitter should not exceed base delay");
        }
    }

    #[tokio::test]
    async fn test_should_retry_predicate() {
        let policy = RetryPolicy::builder()
            .max_attempts(5)
            .backoff(Backoff::constant(Duration::from_millis(10)))
            .with_sleeper(InstantSleeper)
            .should_retry(|e: &TestError| e.0.contains("retryable"))
            .build()
            .expect("builder");

        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        // Test with non-retryable error
        let result = policy
            .execute(|| {
                let counter = counter_clone.clone();
                async move {
                    counter.fetch_add(1, Ordering::SeqCst);
                    Err::<(), _>(ResilienceError::Inner(TestError("fatal error".to_string())))
                }
            })
            .await;

        assert!(result.is_err());
        assert_eq!(counter.load(Ordering::SeqCst), 1, "Should not retry non-retryable error");

        // Reset counter
        counter.store(0, Ordering::SeqCst);

        // Test with retryable error
        let result = policy
            .execute(|| {
                let counter = counter_clone.clone();
                async move {
                    let attempt = counter.fetch_add(1, Ordering::SeqCst);
                    if attempt < 2 {
                        Err(ResilienceError::Inner(TestError("retryable error".to_string())))
                    } else {
                        Ok(42)
                    }
                }
            })
            .await;

        assert_eq!(result.unwrap(), 42);
        assert_eq!(counter.load(Ordering::SeqCst), 3, "Should retry retryable error");
    }

    #[tokio::test]
    async fn test_max_attempts_config() {
        let policy = RetryPolicy::builder()
            .max_attempts(1)
            .with_sleeper(InstantSleeper)
            .build()
            .expect("builder");

        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let result = policy
            .execute(|| {
                let counter = counter_clone.clone();
                async move {
                    counter.fetch_add(1, Ordering::SeqCst);
                    Err::<(), _>(ResilienceError::Inner(TestError("fail".to_string())))
                }
            })
            .await;

        assert!(result.is_err());
        assert_eq!(counter.load(Ordering::SeqCst), 1, "Should only attempt once");
    }

    #[tokio::test]
    async fn test_resilience_error_not_retried() {
        let policy = RetryPolicy::builder()
            .max_attempts(5)
            .backoff(Backoff::constant(Duration::from_millis(10)))
            .with_sleeper(InstantSleeper)
            .build()
            .expect("builder");

        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        // Timeout errors should not be retried (they're not Inner errors)
        let result = policy
            .execute(|| {
                let counter = counter_clone.clone();
                async move {
                    counter.fetch_add(1, Ordering::SeqCst);
                    Err::<(), ResilienceError<TestError>>(ResilienceError::Timeout {
                        elapsed: Duration::from_secs(5),
                        timeout: Duration::from_secs(3),
                    })
                }
            })
            .await;

        assert!(result.is_err());
        assert_eq!(counter.load(Ordering::SeqCst), 1, "Should not retry non-Inner errors");
        assert!(result.unwrap_err().is_timeout());
    }

    #[tokio::test]
    async fn test_exponential_backoff_with_jitter() {
        let sleeper = TrackingSleeper::new();
        let policy = RetryPolicy::builder()
            .max_attempts(4)
            .backoff(Backoff::exponential(Duration::from_millis(100)))
            .with_jitter(Jitter::None)
            .with_sleeper(sleeper.clone())
            .build()
            .expect("builder");

        let _ = policy
            .execute(|| async {
                Err::<(), _>(ResilienceError::Inner(TestError("fail".to_string())))
            })
            .await;

        assert_eq!(sleeper.calls(), 3);

        // Exponential: 100ms, 200ms, 400ms
        assert_eq!(sleeper.call_at(0).unwrap(), Duration::from_millis(100));
        assert_eq!(sleeper.call_at(1).unwrap(), Duration::from_millis(200));
        assert_eq!(sleeper.call_at(2).unwrap(), Duration::from_millis(400));
    }

    #[tokio::test]
    async fn builder_rejects_zero_attempts() {
        let err = RetryPolicy::<TestError>::builder().max_attempts(0).build();
        assert!(matches!(err, Err(BuildError::InvalidMaxAttempts(0))));
    }

    #[tokio::test]
    async fn should_retry_false_short_circuits() {
        let policy = RetryPolicy::builder()
            .max_attempts(5)
            .backoff(Backoff::constant(Duration::from_millis(1)))
            .with_jitter(Jitter::None)
            .should_retry(|_| false)
            .with_sleeper(InstantSleeper)
            .build()
            .expect("builder");

        let attempts = Arc::new(AtomicUsize::new(0));
        let attempts_clone = attempts.clone();

        let result = policy
            .execute(|| {
                let attempts = attempts_clone.clone();
                async move {
                    attempts.fetch_add(1, Ordering::SeqCst);
                    Err::<(), _>(ResilienceError::Inner(TestError("nope".into())))
                }
            })
            .await;

        assert!(matches!(result, Err(ResilienceError::Inner(_))));
        assert_eq!(attempts.load(Ordering::SeqCst), 1, "should not retry");
    }

    // end of tests module
}

// end of file

/// Tower-native retry layer.
pub struct RetryLayer<E> {
    max_attempts: usize,
    backoff: Backoff,
    jitter: Jitter,
    should_retry: Arc<dyn Fn(&E) -> bool + Send + Sync>,
    sleeper: Arc<dyn Sleeper>,
}

impl<E> RetryLayer<E>
where
    E: std::error::Error + Send + Sync + 'static,
{
    pub fn new(
        max_attempts: usize,
        backoff: Backoff,
        jitter: Jitter,
        should_retry: Arc<dyn Fn(&E) -> bool + Send + Sync>,
        sleeper: Arc<dyn Sleeper>,
    ) -> Result<Self, BuildError> {
        if max_attempts == 0 {
            return Err(BuildError::InvalidMaxAttempts(0));
        }
        Ok(Self { max_attempts, backoff, jitter, should_retry, sleeper })
    }
}

impl<E> Clone for RetryLayer<E> {
    fn clone(&self) -> Self {
        Self {
            max_attempts: self.max_attempts,
            backoff: self.backoff.clone(),
            jitter: self.jitter.clone(),
            should_retry: self.should_retry.clone(),
            sleeper: self.sleeper.clone(),
        }
    }
}

/// Retry service produced by `RetryLayer`.
#[derive(Clone)]
pub struct RetryService<S, E> {
    inner: S,
    layer: RetryLayer<E>,
}

impl<S, E> RetryService<S, E> {
    fn new(inner: S, layer: RetryLayer<E>) -> Self {
        Self { inner, layer }
    }
}

impl<S, E, Request> Service<Request> for RetryService<S, E>
where
    Request: Clone + Send + 'static,
    S: Service<Request> + Clone + Send + 'static,
    S::Response: Send + 'static,
    S::Error: std::error::Error + Send + Sync + 'static,
    S::Future: Send + 'static,
    E: std::error::Error + Send + Sync + 'static,
    S::Error: Into<E>,
{
    type Response = S::Response;
    type Error = ResilienceError<E>;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx).map_err(|e| ResilienceError::Inner(e.into()))
    }

    fn call(&mut self, req: Request) -> Self::Future {
        let layer = self.layer.clone();
        let mut inner = self.inner.clone();
        Box::pin(async move {
            let mut failures: Vec<E> = Vec::new();
            for attempt in 0..layer.max_attempts {
                match inner.call(req.clone()).await {
                    Ok(resp) => return Ok(resp),
                    Err(err) => {
                        let e: E = err.into();
                        if !(layer.should_retry)(&e) {
                            return Err(ResilienceError::Inner(e));
                        }
                        failures.push(e);
                        if attempt + 1 >= layer.max_attempts {
                            return Err(ResilienceError::retry_exhausted(
                                layer.max_attempts,
                                failures,
                            ));
                        }
                        let mut delay = layer.backoff.delay(attempt + 1);
                        delay = match &layer.jitter {
                            Jitter::Decorrelated(_) => layer.jitter.apply_stateful(),
                            _ => layer.jitter.apply(delay),
                        };
                        layer.sleeper.sleep(delay).await;
                    }
                }
            }
            Err(ResilienceError::retry_exhausted(layer.max_attempts, failures))
        })
    }
}

impl<S, E> Layer<S> for RetryLayer<E>
where
    E: std::error::Error + Send + Sync + 'static,
{
    type Service = RetryService<S, E>;
    fn layer(&self, service: S) -> Self::Service {
        RetryService::new(service, self.clone())
    }
}
