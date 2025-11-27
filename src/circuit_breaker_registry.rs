use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use crate::circuit_breaker::{CircuitBreakerState, CircuitState};

/// Handle to reset/query a circuit breaker instance.
#[derive(Clone)]
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
#[derive(Default, Clone)]
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
}

static GLOBAL_REGISTRY: OnceLock<CircuitBreakerRegistry> = OnceLock::new();

pub fn global() -> &'static CircuitBreakerRegistry {
    GLOBAL_REGISTRY.get_or_init(CircuitBreakerRegistry::default)
}

pub(crate) fn register_global(id: String, state: Arc<CircuitBreakerState>) {
    let handle = CircuitBreakerHandle { state };
    global().register(id, handle);
}

/// Convenience: create and register a fresh state with the given id.
pub fn register_new(id: String) {
    let state = Arc::new(CircuitBreakerState::new());
    register_global(id, state);
}

/// Read-only state lookup for a breaker id.
pub fn state_of(id: &str) -> Option<CircuitState> {
    global().get(id).map(|h| h.state())
}
