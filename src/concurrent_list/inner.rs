use super::chunk::Chunk;


pub struct ConcurrentListInner<T> {
    pub ownership_chunk: *mut Chunk<T>
}

unsafe impl<T> Sync for ConcurrentListInner<T> {}
unsafe impl<T> Send for ConcurrentListInner<T> {}

impl<T> Default for ConcurrentListInner<T> {
    fn default() -> Self {
        Self::new(super::concurrent_list::DEFAULT_CHUNK_SIZE) // just an ok capacity both in efficeny and performance
    }
}

impl<T> ConcurrentListInner<T> {
    pub fn new(chunk_cap: usize) -> Self {
        Self::with_capacity(chunk_cap, 1)
    }

    pub fn with_capacity(chunk_cap: usize, chunks_count: usize) -> Self {
        assert!(chunks_count > 0);

        let b = unsafe {
            Box::<Chunk<T>>::new(Chunk::new(None, chunk_cap))
        };

        // Create all the chunks
        let mut last_chunk = b.as_ref();
        for _ in 1..chunks_count {
            unsafe {
                let new_box = Box::<Chunk<T>>::new(Chunk::new(Some(last_chunk), chunk_cap));
                last_chunk = Box::into_raw(new_box).as_ref().unwrap();
            }
        }

        // Update pointers
        let mut last_chunk = b.as_ref();
        loop {
            unsafe { last_chunk.update_neighbour_ptrs() };
            if let Some(next) = unsafe { last_chunk.next_node() } {
                last_chunk = next;
                continue;
            }
            break;
        }

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