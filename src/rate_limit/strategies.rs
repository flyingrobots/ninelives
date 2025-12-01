use crate::rate_limit::{RateLimiter, Decision};
use crate::rate_limit::store::TokenStore;
use crate::adaptive::Adaptive;
use async_trait::async_trait;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use std::sync::Arc;

/// A Token Bucket rate limiter.
///
/// Replenishes tokens at a fixed `rate` per second, up to `capacity`.
pub struct TokenBucket<S> {
    store: Arc<S>,
    bucket_key: String,
    rate: Adaptive<f64>, // Tokens per second
    capacity: Adaptive<f64>, // Max tokens
}

impl<S> TokenBucket<S>
where
    S: TokenStore + Send + Sync + 'static,
{
    /// Create a new TokenBucket backed by `store`.
    pub fn new(store: S, key: impl Into<String>, rate: f64, capacity: f64) -> Self {
        Self {
            store: Arc::new(store),
            bucket_key: key.into(),
            rate: Adaptive::new(rate),
            capacity: Adaptive::new(capacity),
        }
    }

    fn now_nanos() -> u64 {
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos() as u64
    }
}

#[async_trait]
impl<S> RateLimiter for TokenBucket<S>
where
    S: TokenStore + Send + Sync + 'static,
{
    async fn acquire(&self, permits: u32) -> Result<Decision, Box<dyn std::error::Error + Send + Sync>> {
        let now = Self::now_nanos();
        let cost = permits as f64;
        let rate = *self.rate.get();
        let capacity = *self.capacity.get();

        // Optimistic locking loop
        for _ in 0..3 { // Try 3 times
            let (current_tokens, last_updated) = match self.store.get_state(&self.bucket_key).await? {
                Some((t, u)) => (t, u),
                None => (capacity, now), // Initial state: full bucket
            };

            // Refill
            let elapsed_secs = (now.saturating_sub(last_updated) as f64) / 1_000_000_000.0;
            let new_tokens = (current_tokens + elapsed_secs * rate).min(capacity);

            if new_tokens >= cost {
                let final_tokens = new_tokens - cost;
                // Try to commit
                if self.store.set_state(&self.bucket_key, final_tokens, now, Some(last_updated)).await? {
                    return Ok(Decision::Allowed {
                        remaining: final_tokens as u32,
                        metadata: Default::default(),
                    });
                }
                // Race detected, loop again
            } else {
                // Not enough tokens. Calculate wait time.
                let missing = cost - new_tokens;
                let wait_secs = missing / rate;
                return Ok(Decision::Denied {
                    wait: Duration::from_secs_f64(wait_secs),
                    reason: "token_bucket_empty".into(),
                });
            }
        }

        // Failed to acquire lock after retries
        // In a real system, we might deny or fail open.
        Ok(Decision::Denied {
            wait: Duration::from_millis(100), // Arbitrary backoff on contention
            reason: "store_contention".into(),
        })
    }
}
