use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

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