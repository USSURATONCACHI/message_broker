use std::sync::Arc;

use super::{chunk::Chunk, ChunkRef};

pub struct ConcurrentList<T> {
    ownership_chunk: *mut Chunk<T>
}

unsafe impl<T> Sync for ConcurrentList<T> {}
unsafe impl<T> Send for ConcurrentList<T> {}

impl<T> ConcurrentList<T> {
    pub fn new(cap: usize) -> Self {
        let b = Box::<Chunk<T>>::new(Chunk::new(None, cap));

        let raw = Box::into_raw(b);

        Self {
            ownership_chunk: raw,
        }
    }

    pub fn reference(self: &Arc<Self>) -> ChunkRef<T> {
        unsafe {
            ChunkRef::new_at(self.clone(), self.ownership_chunk.as_ref().unwrap(), 0).unwrap()
        }
    }

    pub fn len(&self) -> usize {
        unsafe {
            self.chunk().front_elems_count() + self.chunk().back_elems_count()
        }
    }

    pub fn nodes_count(&self) -> usize {
        unsafe {
            self.chunk().front_nodes_count() + self.chunk().back_nodes_count()
        }
    }

    fn chunk(&self) -> &Chunk<T> {
        unsafe { self.ownership_chunk.as_ref().unwrap() }
    }
}

impl<T> Drop for ConcurrentList<T> {
    fn drop(&mut self) {
        unsafe {
            Box::from_raw(self.ownership_chunk).drop_all_links();
        }
    }
}