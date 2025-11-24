//! Resilience stack builder for composing policies

use crate::{BulkheadPolicy, CircuitBreakerConfig, CircuitBreakerPolicy, ResilienceError, RetryPolicy, TimeoutPolicy};
use std::future::Future;
use std::time::Duration;

#[derive(Clone)]
pub struct ResilienceStack<E> {
    pub(crate) timeout: TimeoutPolicy,
    pub(crate) bulkhead: BulkheadPolicy,
    pub(crate) circuit_breaker: CircuitBreakerPolicy,
    pub(crate) retry: RetryPolicy<E>,
}

impl<E> ResilienceStack<E>
where
    E: std::error::Error + Send + Sync + 'static,
{
    pub fn new() -> ResilienceStackBuilder<E> {
        ResilienceStackBuilder::new()
    }

    pub async fn execute<T, Fut, Op>(&self, operation: Op) -> Result<T, ResilienceError<E>>
    where
        T: Send,
        Fut: Future<Output = Result<T, ResilienceError<E>>> + Send,
        Op: FnMut() -> Fut + Send,
    {
        // Stack order: Retry → CircuitBreaker → Bulkhead → Timeout → Operation
        // Each layer wraps the next, building from inside out

        // Use Arc<Mutex<>> for Send + Sync interior mutability
        use std::sync::{Arc, Mutex};

        let op_cell = Arc::new(Mutex::new(operation));

        self.retry
            .execute(|| {
                let op = op_cell.clone();
                let circuit_breaker = self.circuit_breaker.clone();
                let bulkhead = self.bulkhead.clone();
                let timeout = self.timeout.clone();

                async move {
                    circuit_breaker
                        .execute(|| {
                            let op = op.clone();
                            let bulkhead = bulkhead.clone();
                            let timeout = timeout.clone();
                            async move {
                                bulkhead
                                    .execute(|| {
                                        let op = op.clone();
                                        let timeout = timeout.clone();
                                        async move {
                                            timeout
                                                .execute(|| {
                                                    let mut op = op.lock().unwrap();
                                                    op()
                                                })
                                                .await
                                        }
                                    })
                                    .await
                            }
                        })
                        .await
                }
            })
            .await
    }
}

impl<E> Default for ResilienceStack<E>
where
    E: std::error::Error + Send + Sync + 'static,
{
    fn default() -> Self {
        ResilienceStackBuilder::new().build()
    }
}

pub struct ResilienceStackBuilder<E> {
    timeout: Option<TimeoutPolicy>,
    bulkhead: Option<BulkheadPolicy>,
    circuit_breaker: Option<CircuitBreakerPolicy>,
    retry: Option<RetryPolicy<E>>,
}

impl<E> ResilienceStackBuilder<E>
where
    E: std::error::Error + Send + Sync + 'static,
{
    pub fn new() -> Self {
        Self {
            timeout: None,
            bulkhead: None,
            circuit_breaker: None,
            retry: None,
        }
    }

    pub fn timeout(mut self, duration: Duration) -> Self {
        self.timeout = Some(TimeoutPolicy::new(duration));
        self
    }

    pub fn no_timeout(mut self) -> Self {
        self.timeout = Some(TimeoutPolicy::new(Duration::from_secs(u64::MAX / 1000)));
        self
    }

    pub fn bulkhead(mut self, max_concurrent: usize) -> Self {
        self.bulkhead = Some(BulkheadPolicy::new(max_concurrent));
        self
    }

    pub fn unlimited_bulkhead(mut self) -> Self {
        self.bulkhead = Some(BulkheadPolicy::unlimited());
        self
    }

    pub fn circuit_breaker(mut self, failures: usize, timeout: Duration) -> Self {
        self.circuit_breaker = Some(CircuitBreakerPolicy::new(failures, timeout));
        self
    }

    pub fn circuit_breaker_with_config(mut self, config: CircuitBreakerConfig) -> Self {
        self.circuit_breaker = Some(CircuitBreakerPolicy::with_config(config));
        self
    }

    pub fn no_circuit_breaker(mut self) -> Self {
        self.circuit_breaker = Some(CircuitBreakerPolicy::with_config(
            CircuitBreakerConfig::disabled(),
        ));
        self
    }

    pub fn retry(mut self, policy: RetryPolicy<E>) -> Self {
        self.retry = Some(policy);
        self
    }

    pub fn build(self) -> ResilienceStack<E> {
        ResilienceStack {
            timeout: self
                .timeout
                .unwrap_or_else(|| TimeoutPolicy::new(Duration::from_secs(30))),
            bulkhead: self.bulkhead.unwrap_or_else(|| BulkheadPolicy::new(100)),
            circuit_breaker: self.circuit_breaker.unwrap_or_else(|| {
                CircuitBreakerPolicy::new(5, Duration::from_secs(60))
            }),
            retry: self.retry.unwrap_or_else(|| RetryPolicy::builder().build()),
        }
    }
}

impl<E> Default for ResilienceStackBuilder<E>
where
    E: std::error::Error + Send + Sync + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}
