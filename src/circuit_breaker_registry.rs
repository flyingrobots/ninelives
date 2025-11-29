//! Registry for managing named circuit breakers.
//!
//! Allows global access and control (reset/inspection) of circuit breakers by ID.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::circuit_breaker::{CircuitBreakerState, CircuitState};

/// Handle to reset/query a circuit breaker instance.
#[derive(Clone, Debug)]
pub struct CircuitBreakerHandle {
    pub(crate) state: Arc<CircuitBreakerState>,
}

impl CircuitBreakerHandle {
    /// Reset the circuit breaker state to Closed, clearing failure counts.
    pub fn reset(&self) {
        self.state.reset();
    }

    /// Current breaker state.
    pub fn state(&self) -> CircuitState {
        self.state.current_state()
    }
}

/// Trait for breaker registries (injectable into control plane).
pub trait CircuitBreakerRegistry: Send + Sync + std::fmt::Debug {
    /// Register or overwrite a circuit breaker handle by id.
    fn register(&self, id: String, handle: CircuitBreakerHandle);
    /// Get a breaker handle by id.
    fn get(&self, id: &str) -> Option<CircuitBreakerHandle>;
    /// Reset a breaker by id, erroring if missing.
    fn reset(&self, id: &str) -> Result<(), String>;
    /// Convenience: create and insert a new breaker state with id.
    fn register_new(&self, id: String);
    /// Snapshot breaker states sorted by id.
    fn snapshot(&self) -> Vec<(String, CircuitState)>;
}

/// In-memory implementation backed by a Mutex.
#[derive(Default, Clone, Debug)]
pub struct InMemoryCircuitBreakerRegistry {
    inner: Arc<RwLock<HashMap<String, CircuitBreakerHandle>>>,
}

pub type DefaultCircuitBreakerRegistry = InMemoryCircuitBreakerRegistry;

impl CircuitBreakerRegistry for InMemoryCircuitBreakerRegistry {
    #[allow(unused_mut)]
    fn register(&self, id: String, handle: CircuitBreakerHandle) {
        let guard = self.inner.write().expect("circuit breaker registry poisoned");
        // allow shadowing to keep scope small
        let mut map = guard;
        map.insert(id, handle);
    }

    fn get(&self, id: &str) -> Option<CircuitBreakerHandle> {
        let guard = self.inner.read().expect("circuit breaker registry poisoned");
        guard.get(id).cloned()
    }

    fn reset(&self, id: &str) -> Result<(), String> {
        let guard = self.inner.write().expect("circuit breaker registry poisoned");
        match guard.get(id) {
            Some(handle) => {
                handle.reset();
                Ok(())
            }
            None => Err(format!("breaker id not found: {id}")),
        }
    }

    fn register_new(&self, id: String) {
        let state = Arc::new(CircuitBreakerState::new());
        let handle = CircuitBreakerHandle { state };
        self.register(id, handle);
    }

    fn snapshot(&self) -> Vec<(String, CircuitState)> {
        let map = self.inner.read().expect("circuit breaker registry poisoned");
        let mut entries: Vec<(String, CircuitState)> =
            map.iter().map(|(k, v)| (k.clone(), v.state())).collect();
        entries.sort_by(|a, b| a.0.cmp(&b.0));
        entries
    }
}
