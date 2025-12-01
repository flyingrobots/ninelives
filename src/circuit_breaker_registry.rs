//! Registry for managing named circuit breakers.
//!
//! Allows global access and control (reset/inspection) of circuit breakers by ID.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::circuit_breaker::{CircuitBreakerState, CircuitState};
use tracing::warn;

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
    /// Register a circuit breaker handle by id, overwriting any existing handle.
    ///
    /// Overwrite is deliberate: when multiple services share an ID, the last
    /// registration wins. Callers should normally use unique IDs per breaker and
    /// treat overwrites as a replacement, not a merge of state.
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
    fn register(&self, id: String, handle: CircuitBreakerHandle) {
        let mut map = self.inner.write().expect("circuit breaker registry poisoned");
        if map.contains_key(&id) {
            warn!(target: "ninelives::circuit_breaker_registry", id = %id, "circuit breaker id replaced; last registration wins");
        }
        map.insert(id, handle);
    }

    fn get(&self, id: &str) -> Option<CircuitBreakerHandle> {
        let guard = self.inner.read().expect("circuit breaker registry poisoned");
        guard.get(id).cloned()
    }

    fn reset(&self, id: &str) -> Result<(), CircuitBreakerRegistryError> {
        let guard = self.inner.read().expect("circuit breaker registry poisoned");
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    use tracing_subscriber::fmt::writer::BoxMakeWriter;
    use tracing_subscriber::fmt::MakeWriter;

    #[derive(Clone)]
    struct SharedWriter(Arc<Mutex<Vec<u8>>>);

    impl<'a> MakeWriter<'a> for SharedWriter {
        type Writer = SharedGuard;
        fn make_writer(&'a self) -> Self::Writer {
            SharedGuard(self.0.clone())
        }
    }

    struct SharedGuard(Arc<Mutex<Vec<u8>>>);
    impl std::io::Write for SharedGuard {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            let mut guard = self.0.lock().unwrap();
            guard.extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn register_warns_and_replaces_duplicates() {
        let buffer = Arc::new(Mutex::new(Vec::new()));
        let writer = SharedWriter(buffer.clone());
        let subscriber = tracing_subscriber::fmt()
            .with_writer(BoxMakeWriter::new(writer))
            .with_target(true)
            .without_time()
            .finish();
        let _guard = tracing::subscriber::set_default(subscriber);

        let registry = InMemoryCircuitBreakerRegistry::default();
        // first registration
        let h1 = CircuitBreakerHandle { state: Arc::new(CircuitBreakerState::new()) };
        registry.register("svc".into(), h1.clone());
        // second registration with same id
        let h2 = CircuitBreakerHandle { state: Arc::new(CircuitBreakerState::new()) };
        registry.register("svc".into(), h2.clone());

        let resolved = registry.get("svc").expect("handle present");
        assert!(Arc::ptr_eq(&resolved.state, &h2.state), "last registration should win");

        let logs = String::from_utf8(buffer.lock().unwrap().clone()).unwrap();
        assert!(
            logs.contains("circuit breaker id replaced"),
            "warning should be emitted on duplicate registration"
        );
    }
}
