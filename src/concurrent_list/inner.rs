use super::chunk::Chunk;


pub struct ConcurrentListInner<T> {
    pub ownership_chunk: *mut Chunk<T>
}

unsafe impl<T> Sync for ConcurrentListInner<T> {}
unsafe impl<T> Send for ConcurrentListInner<T> {}

impl<T> Default for ConcurrentListInner<T> {
    fn default() -> Self {
        Self::new(256) // just an ok capacity both in efficeny and performance
    }
}

impl<T> ConcurrentListInner<T> {
    pub fn new(cap: usize) -> Self {
        let b = Box::<Chunk<T>>::new(Chunk::new(None, cap));

        let raw = Box::into_raw(b);

        Self {
            ownership_chunk: raw,
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

impl<T> Drop for ConcurrentListInner<T> {
    fn drop(&mut self) {
        unsafe {
            Box::from_raw(self.ownership_chunk).drop_all_links();
        }
    }
}