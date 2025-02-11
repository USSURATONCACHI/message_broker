use std::{ops::{Deref, DerefMut}, sync::RwLockWriteGuard};


pub struct ChunkWriteGuard<'a, T> {
    inner_lock: RwLockWriteGuard<'a, Option<T>>
}
impl<'a, T> ChunkWriteGuard<'a, T> {
    pub fn from_rwlock(inner_lock: RwLockWriteGuard<'a, Option<T>>) -> Self {
        Self { inner_lock }
    }

    pub fn deref_option(&self) -> &Option<T> {
        self.inner_lock.deref()
    }
    pub fn deref_option_mut(&mut self) -> &mut Option<T> {
        self.inner_lock.deref_mut()
    }
}

impl<'a, T> Deref for ChunkWriteGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.inner_lock.deref().as_ref().unwrap()
    }
}

impl<'a, T> DerefMut for ChunkWriteGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner_lock.deref_mut().as_mut().unwrap()
    }
}