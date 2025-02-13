use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

/// A thread-safe, shared handle to a value protected by a read-write lock.
///
/// Cloning creates new references to the same data. Uses [`RwLock`] for interior mutability.
///
/// # Examples
///
/// Basic usage:
/// ```
/// use broker::util::Handle;
///
/// let handle = Handle::<Vec<i32>>::new();
/// handle.get_mut().push(1);
///
/// let handle2 = handle.clone();
/// assert_eq!(handle2.get().len(), 1);
///
/// handle.get_mut().push(2);
/// assert_eq!(handle2.get().len(), 2);
/// ```
pub struct Handle<T>(pub Arc<RwLock<T>>);

impl<T> Clone for Handle<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T: Default> Default for Handle<T> {
    fn default() -> Self {
        Self(Arc::new(RwLock::new(T::default())))
    }
}

impl<T: Default> Handle<T> {
    pub fn new() -> Self {
        Self::default()
    }
}

impl <T> From<T> for Handle<T> {
    fn from(value: T) -> Self {
        Self(Arc::new(RwLock::new(value)))
    }
}

impl<T> Handle<T> {
    pub fn get(&self) -> RwLockReadGuard<T> {
        self.0.read().unwrap()
    }
    pub fn get_mut(&self) -> RwLockWriteGuard<T> {
        self.0.write().unwrap()
    }
}