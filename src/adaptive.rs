//! DynamicConfig handles for live-updatable config.
//!
//! Provides a mechanism for shared, live-updatable configuration values with configurable backends.
//!
//! Two backends are available, controlled by Cargo features:
//!
//! -   **Default (no feature)**: Uses `ArcSwap` for lock-free atomic reads and updates.
//!     *   **Read Performance**: Extremely cheap (pointer copy).
//!     *   **Write Performance**: `compare_and_swap` loop (optimistic concurrency), retries on collision until success; concurrent `update()` calls do not lose intermediate states (see `concurrent_updates_no_lost_updates` test) but high contention can cause more retries.
//!     *   **Thread-Safety**: Lock-free, atomic.
//!     *   **Lock Poisoning**: Not applicable.
//!
//! -   **`adaptive-rwlock` feature**: Uses `std::sync::RwLock<Arc<T>>`.
//!     *   **Read Performance**: Requires acquiring a read lock, cloning an `Arc<T>`. More expensive than `ArcSwap` due to locking overhead and `Arc` clone.
//!     *   **Write Performance**: Requires acquiring a write lock, performing the update. Writes are serialized.
//!     *   **Thread-Safety**: Read/write locking. `update()` calls are serialized by the write lock; follows `RwLock` poisoning semantics.
//!     *   **Lock Poisoning**: Follows `std::sync::RwLock` poisoning semantics. If a thread panics while holding a write lock, the lock becomes poisoned. Subsequent lock acquisitions will return an error (which is currently handled by `expect()`, causing panic).
//!
//! Choose the backend that best fits your performance and concurrency profile.

use std::sync::Arc;

#[cfg(feature = "adaptive-rwlock")]
use std::sync::RwLock;

#[cfg(not(feature = "adaptive-rwlock"))]
use arc_swap::ArcSwap;

/// `DynamicConfig<T>` gives cheap reads and controlled updates for shared config.
#[derive(Debug, Clone)]
pub struct DynamicConfig<T> {
    #[cfg(not(feature = "adaptive-rwlock"))]
    inner: Arc<ArcSwap<T>>,
    #[cfg(feature = "adaptive-rwlock")]
    inner: Arc<RwLock<Arc<T>>>, // Store Arc<T>
}

// Back-compat alias for existing code/tests referencing Adaptive.
/// Alias for `DynamicConfig` for backward compatibility and easier typing.
pub type Adaptive<T> = DynamicConfig<T>;

impl<T> DynamicConfig<T> {
    /// Create a new `DynamicConfig` with the given initial value.
    pub fn new(value: T) -> Self {
        #[cfg(not(feature = "adaptive-rwlock"))]
        {
            Self { inner: Arc::new(ArcSwap::from_pointee(value)) }
        }
        #[cfg(feature = "adaptive-rwlock")]
        {
            Self { inner: Arc::new(RwLock::new(Arc::new(value))) } // Wrap initial value in Arc
        }
    }

    /// Snapshot the current value (cheap clone of Arc).
    #[cfg(not(feature = "adaptive-rwlock"))]
    pub fn get(&self) -> Arc<T> {
        self.inner.load_full()
    }

    /// Snapshot the current value (Clone under RwLock backend).
    #[cfg(feature = "adaptive-rwlock")]
    pub fn get(&self) -> Arc<T> {
        self.inner.read().expect("adaptive config lock poisoned").clone() // Clone the inner Arc<T>
    }

    /// Replace the value entirely.
    pub fn set(&self, value: T) {
        #[cfg(not(feature = "adaptive-rwlock"))]
        {
            self.inner.store(Arc::new(value));
        }
        #[cfg(feature = "adaptive-rwlock")]
        {
            *self.inner.write().expect("adaptive config lock poisoned") = Arc::new(value);
            // Store Arc<T>
        }
    }

    /// Update via closure.
    ///
    /// Notes:
    /// - ArcSwap backend: `f` may run multiple times per call under contention due to the CAS loop;
    ///   it must be pure/side-effect free and reasonably cheap. CAS retries until success, so
    ///   concurrent updates are not lost.
    /// - `adaptive-rwlock` backend: `f` runs while holding the write lock; keep it fast and do not
    ///   re-enter this `DynamicConfig` from within `f`. Standard RwLock poisoning semantics apply.
    pub fn update<F>(&self, f: F)
    where
        F: Fn(&T) -> T,
    {
        #[cfg(not(feature = "adaptive-rwlock"))]
        {
            loop {
                let cur = self.inner.load_full();
                let new_val = Arc::new(f(&cur));
                let prev = self.inner.compare_and_swap(&cur, new_val.clone());
                // If CAS succeeded, the previous value matches the one we saw.
                if Arc::ptr_eq(&prev, &cur) {
                    break;
                }
                // CAS failed, retry with new current value
            }
        }
        #[cfg(feature = "adaptive-rwlock")]
        {
            let mut guard = self.inner.write().expect("adaptive config lock poisoned");
            let new_val = f(&guard);
            *guard = Arc::new(new_val);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::DynamicConfig;
    use std::sync::Arc as StdArc;
    use std::thread;

    #[test]
    fn get_set_update() {
        let a = DynamicConfig::new(1);
        assert_eq!(*a.get(), 1);
        a.set(2);
        assert_eq!(*a.get(), 2);
        a.update(|v| v + 3);
        assert_eq!(*a.get(), 5);
    }

    #[test]
    fn concurrent_updates_no_lost_updates() {
        const NUM_THREADS: usize = 10;
        const NUM_UPDATES_PER_THREAD: usize = 1_000;
        let config = StdArc::new(DynamicConfig::new(0));

        let handles: Vec<_> = (0..NUM_THREADS)
            .map(|_| {
                let config_clone = StdArc::clone(&config);
                thread::spawn(move || {
                    for _ in 0..NUM_UPDATES_PER_THREAD {
                        config_clone.update(|v| v + 1);
                    }
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }

        assert_eq!(*config.get(), NUM_THREADS * NUM_UPDATES_PER_THREAD);
    }

    #[test]
    fn concurrent_get_set() {
        const NUM_ITERATIONS: usize = 1_000;
        let config = StdArc::new(DynamicConfig::new(vec![1, 2, 3]));

        let mut handles = vec![];

        // Writer threads
        for i in 0i32..2 {
            let config_clone = StdArc::clone(&config);
            handles.push(thread::spawn(move || {
                for j in 0..NUM_ITERATIONS as i32 {
                    config_clone.set(vec![i, j]);
                }
            }));
        }

        // Reader threads
        for _ in 0..3 {
            let config_clone = StdArc::clone(&config);
            handles.push(thread::spawn(move || {
                for _ in 0..NUM_ITERATIONS {
                    let _ = config_clone.get(); // Just read, ensure no panics
                }
            }));
        }

        for handle in handles {
            handle.join().unwrap(); // Ensure all threads complete without panic
        }
        let _ = config.get(); // Final read
    }

    #[cfg(feature = "adaptive-rwlock")]
    #[test]
    fn rwlock_get_returns_shared_arc() {
        let config = DynamicConfig::new(42);
        let first_arc = config.get();
        let second_arc = config.get();

        // Under the adaptive-rwlock backend, get() should return Arcs pointing to the same
        // underlying value. This asserts pointer equality to catch regressions.
        assert!(StdArc::ptr_eq(&first_arc, &second_arc));
    }
}
