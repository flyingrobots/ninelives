//! Adaptive handles for live-updatable config.
//!
//! Default uses `ArcSwap` for lock-free reads; feature `adaptive-rwlock` can
//! switch to RwLock if desired.

use std::sync::Arc;

#[cfg(feature = "adaptive-rwlock")]
use std::sync::RwLock;

#[cfg(not(feature = "adaptive-rwlock"))]
use arc_swap::ArcSwap;

/// Adaptive<T> gives cheap reads and controlled updates for shared config.
#[derive(Debug)]
pub struct Adaptive<T> {
    #[cfg(not(feature = "adaptive-rwlock"))]
    inner: Arc<ArcSwap<T>>,
    #[cfg(feature = "adaptive-rwlock")]
    inner: Arc<RwLock<T>>,
}

impl<T> Clone for Adaptive<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T> Adaptive<T> {
    pub fn new(value: T) -> Self {
        #[cfg(not(feature = "adaptive-rwlock"))]
        {
            Self {
                inner: Arc::new(ArcSwap::from_pointee(value)),
            }
        }
        #[cfg(feature = "adaptive-rwlock")]
        {
            Self {
                inner: Arc::new(RwLock::new(value)),
            }
        }
    }

    /// Snapshot the current value (cheap clone of Arc).
    pub fn get(&self) -> Arc<T> {
        #[cfg(not(feature = "adaptive-rwlock"))]
        {
            self.inner.load_full()
        }
        #[cfg(feature = "adaptive-rwlock")]
        {
            Arc::new(self.inner.read().unwrap().clone())
        }
    }

    /// Replace the value entirely.
    pub fn set(&self, value: T) {
        #[cfg(not(feature = "adaptive-rwlock"))]
        {
            self.inner.store(Arc::new(value));
        }
        #[cfg(feature = "adaptive-rwlock")]
        {
            *self.inner.write().unwrap() = value;
        }
    }

    /// Update via closure.
    pub fn update<F>(&self, f: F)
    where
        F: FnOnce(&T) -> T,
        T: Clone,
    {
        #[cfg(not(feature = "adaptive-rwlock"))]
        {
            let cur = self.inner.load_full();
            let new_val = f(&cur);
            self.inner.store(Arc::new(new_val));
        }
        #[cfg(feature = "adaptive-rwlock")]
        {
            let cur = self.inner.read().unwrap().clone();
            let new_val = f(&cur);
            *self.inner.write().unwrap() = new_val;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Adaptive;

    #[test]
    fn get_set_update() {
        let a = Adaptive::new(1);
        assert_eq!(*a.get(), 1);
        a.set(2);
        assert_eq!(*a.get(), 2);
        a.update(|v| v + 3);
        assert_eq!(*a.get(), 5);
    }
}
