use std::{ops::Deref, sync::RwLockReadGuard};


pub struct ChunkReadGuard<'a, T> {
    inner_lock: RwLockReadGuard<'a, Option<T>>
}
impl<'a, T> ChunkReadGuard<'a, T> {
    pub fn from_rwlock_read(inner_lock: RwLockReadGuard<'a, Option<T>>) -> Self {
        Self { inner_lock }
    }
}
impl<'a, T> Deref for ChunkReadGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.inner_lock.deref().as_ref().unwrap()
    }
}
