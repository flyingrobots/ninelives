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

/// Errors from breaker registries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CircuitBreakerRegistryError {
    /// The requested circuit breaker ID was not found.
    NotFound {
        /// Identifier that could not be located.
        id: String,
    },
}

impl std::fmt::Display for CircuitBreakerRegistryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CircuitBreakerRegistryError::NotFound { id } => {
                write!(f, "circuit breaker '{id}' not found")
            }
        }
    }
}

impl std::error::Error for CircuitBreakerRegistryError {}

/// Trait for breaker registries (injectable into control plane).
pub trait CircuitBreakerRegistry: Send + Sync + std::fmt::Debug {
    /// Register or overwrite a circuit breaker handle by id.
    fn register(&self, id: String, handle: CircuitBreakerHandle);
    /// Get a breaker handle by id.
    fn get(&self, id: &str) -> Option<CircuitBreakerHandle>;
    /// Reset a breaker by id, erroring if missing.
    fn reset(&self, id: &str) -> Result<(), CircuitBreakerRegistryError>;
    /// Convenience: create and insert a new breaker state with id.
    fn register_new(&self, id: String);
    /// Snapshot breaker states sorted by id.
    fn snapshot(&self) -> Vec<(String, CircuitState)>;
}

/// In-memory implementation backed by an RwLock.
#[derive(Default, Clone, Debug)]
pub struct InMemoryCircuitBreakerRegistry {
    inner: Arc<RwLock<HashMap<String, CircuitBreakerHandle>>>,
}

/// Default registry used when none is injected.
pub type DefaultCircuitBreakerRegistry = InMemoryCircuitBreakerRegistry;

impl CircuitBreakerRegistry for InMemoryCircuitBreakerRegistry {
    #[allow(unused_mut)]
    fn register(&self, id: String, handle: CircuitBreakerHandle) {
        let guard = self.inner.write().expect("circuit breaker registry poisoned");
        let mut map = guard;
        map.insert(id, handle);
    }

    fn get(&self, id: &str) -> Option<CircuitBreakerHandle> {
        let guard = self.inner.read().expect("circuit breaker registry poisoned");
        guard.get(id).cloned()
    }

    fn reset(&self, id: &str) -> Result<(), CircuitBreakerRegistryError> {
        let guard = self.inner.write().expect("circuit breaker registry poisoned");
        match guard.get(id) {
            Some(handle) => {
                handle.reset();
                Ok(())
            }
            None => Err(CircuitBreakerRegistryError::NotFound { id: id.to_string() }),
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
