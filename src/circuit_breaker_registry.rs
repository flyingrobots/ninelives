use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use crate::circuit_breaker::CircuitBreakerState;

/// Handle to reset/query a circuit breaker instance.
#[derive(Clone)]
pub struct CircuitBreakerHandle {
    pub(crate) state: Arc<CircuitBreakerState>,
}

impl CircuitBreakerHandle {
    pub fn reset(&self) {
        self.state.reset();
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
}

static GLOBAL_REGISTRY: OnceLock<CircuitBreakerRegistry> = OnceLock::new();

pub fn global() -> &'static CircuitBreakerRegistry {
    GLOBAL_REGISTRY.get_or_init(CircuitBreakerRegistry::default)
}

pub fn register_global(id: String, state: Arc<CircuitBreakerState>) {
    let handle = CircuitBreakerHandle { state };
    global().register(id, handle);
}
