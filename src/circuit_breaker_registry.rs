use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::circuit_breaker::{CircuitBreakerState, CircuitState};

/// Handle to reset/query a circuit breaker instance.
#[derive(Clone, Debug)]
pub struct CircuitBreakerHandle {
    pub(crate) state: Arc<CircuitBreakerState>,
}

impl CircuitBreakerHandle {
    pub fn reset(&self) {
        self.state.reset();
    }

    /// Current breaker state.
    pub fn state(&self) -> CircuitState {
        self.state.current_state()
    }
}

/// Registry keyed by breaker id.
#[derive(Default, Clone, Debug)]
pub struct CircuitBreakerRegistry {
    inner: Arc<Mutex<HashMap<String, CircuitBreakerHandle>>>,
}

impl CircuitBreakerRegistry {
    pub fn register(&self, id: String, handle: CircuitBreakerHandle) {
        self.inner.lock().unwrap().insert(id, handle);
    }

    pub fn get(&self, id: &str) -> Option<CircuitBreakerHandle> {
        self.inner.lock().unwrap().get(id).cloned()
    }

    pub fn reset(&self, id: &str) -> Result<(), String> {
        if let Some(handle) = self.get(id) {
            handle.reset();
            Ok(())
        } else {
            Err(format!("breaker id not found: {id}"))
        }
    }

    /// Convenience: create and register a fresh state with the given id.
    pub fn register_new(&self, id: String) {
        let state = Arc::new(CircuitBreakerState::new());
        let handle = CircuitBreakerHandle { state };
        self.register(id, handle);
    }

    /// Snapshot of all breaker states (id -> state).
    pub fn snapshot(&self) -> Vec<(String, CircuitState)> {
        let map = self.inner.lock().unwrap();
        let mut entries: Vec<(String, CircuitState)> =
            map.iter().map(|(k, v)| (k.clone(), v.state())).collect();
        entries.sort_by(|a, b| a.0.cmp(&b.0));
        entries
    }
}
