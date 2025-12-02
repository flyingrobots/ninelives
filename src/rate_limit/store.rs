use async_trait::async_trait;

/// Abstract storage interface for rate limit state (e.g., tokens).
///
/// This trait is designed to support both in-memory and distributed backends.
/// It assumes a key-value model where the value is numeric (tokens).
#[async_trait]
pub trait TokenStore: Send + Sync {
    /// Error type for storage operations.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Fetch the current state for a key.
    ///
    /// Returns `(tokens, last_updated_nanos)`.
    async fn get_state(&self, key: &str) -> Result<Option<(f64, u64)>, Self::Error>;

    /// Update the state for a key using a "Compare-And-Set" (CAS) semantic or atomic overwrite.
    ///
    /// * `key`: The bucket identifier.
    /// * `tokens`: The new token count.
    /// * `updated_at`: The new timestamp (nanos).
    /// * `prev_updated_at`: The previous timestamp read (optimistic locking).
    ///   If `None`, implies unconditional write (or first write).
    ///
    /// Returns `Ok(true)` if update succeeded, `Ok(false)` if race detected (retry needed).
    async fn set_state(
        &self,
        key: &str,
        tokens: f64,
        updated_at: u64,
        prev_updated_at: Option<u64>,
    ) -> Result<bool, Self::Error>;
}

use std::sync::{Arc, Mutex};
use std::collections::HashMap;

/// Simple in-memory token store.
#[derive(Default, Clone, Debug)]
pub struct InMemoryTokenStore {
    // Map key -> (tokens, last_updated_nanos)
    data: Arc<Mutex<HashMap<String, (f64, u64)>>>,
}

impl InMemoryTokenStore {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl TokenStore for InMemoryTokenStore {
    type Error = std::convert::Infallible;

    async fn get_state(&self, key: &str) -> Result<Option<(f64, u64)>, Self::Error> {
        let guard = self.data.lock().unwrap();
        Ok(guard.get(key).cloned())
    }

    async fn set_state(
        &self,
        key: &str,
        tokens: f64,
        updated_at: u64,
        prev_updated_at: Option<u64>,
    ) -> Result<bool, Self::Error> {
        let mut guard = self.data.lock().unwrap();
        
        if let Some(prev) = prev_updated_at {
            // Optimistic lock check
            if let Some(&(_, current_ts)) = guard.get(key) {
                if current_ts != prev {
                    return Ok(false); // Race detected
                }
            } else {
                // Key didn't exist, but we expected 'prev'. 
                // In TokenBucket logic, if get_state returned None, we use 'now' as prev.
                // If key now exists, it's a race.
                if guard.contains_key(key) {
                     return Ok(false);
                }
            }
        }

        guard.insert(key.to_string(), (tokens, updated_at));
        Ok(true)
    }
}
