use std::sync::{Arc, RwLockReadGuard, RwLockWriteGuard};

use super::{chunk::Chunk, inner::ConcurrentListInner};

#[derive(Clone)]
pub struct ChunkRef<T: 'static> {
    pub chunk: *const Chunk<T>, // Quite unsafe, be careful
    _ownership_insurance: Arc<ConcurrentListInner<T>>,

    index: usize,
    global_index: usize,

    item_owed: bool,
}
unsafe impl<T> Sync for ChunkRef<T> {}
unsafe impl<T> Send for ChunkRef<T> {}

pub type EndOfCollection = ();

impl<T: 'static> ChunkRef<T> {
    pub unsafe fn new_at(ownership_insurance: Arc<ConcurrentListInner<T>>, mut chunk: &Chunk<T>, mut index: usize) -> Option<Self> {
        let global_index = index + chunk.node_start_index();

        while index >= chunk.node_capacity() {
            index -= chunk.node_capacity();

            let next = chunk.next_node();
            match next {
                None => return None,
                Some(next) => chunk = next,
            }
        }

        if index != 0 && index >= chunk.node_len() {
            return None;
        }

        Some(Self {
            chunk,
            index,
            global_index,
            item_owed: true,
            _ownership_insurance: ownership_insurance,
        })
    }

    pub fn chunk(&self) -> &'static Chunk<T> {
        unsafe { self.chunk.as_ref().unwrap() }
    }
    pub unsafe fn set_chunk(&mut self, chunk: &Chunk<T>) {
        self.chunk = chunk;
    }

    pub fn get(&self) -> Option<RwLockReadGuard<'static, Option<T>>> {
        return self.chunk().at(self.index);
    }
    pub fn get_mut(&self) -> Option<RwLockWriteGuard<'static, Option<T>>> {
        return self.chunk().at_mut(self.index);
    }
    pub fn index(&self) -> usize {
        self.global_index
    }

    pub fn go_next(&mut self) -> Result<(), EndOfCollection> {
        if (self.index + 1) == self.chunk().node_capacity() {
            // Go to next chunk if exists
            let next_node = unsafe { self.chunk().next_node() };
            match next_node {
                None => Err(EndOfCollection::default()),
                Some(next) => {
                    self.chunk = next;
                    self.index = 0;
                    self.global_index += 1;
                    Ok(())
                }
            }
        } else if (self.index + 1) < self.chunk().node_len() {
            // Increment index
            self.index += 1;
            self.global_index += 1;
            Ok(())
        } else {
            Err(EndOfCollection::default())
        }
    }
    pub fn go_prev(&mut self) -> Result<(), EndOfCollection> {
        if self.index == 0 {
            // Go to prev chunk if exists
            let prev_node = unsafe { self.chunk().prev_node() };
            match prev_node {
                None => Err(EndOfCollection::default()),
                Some(prev) => {
                    self.index = prev.node_capacity() - 1;
                    self.global_index -= 1;
                    Ok(())
                }
            }
        } else {
            self.index -= 1;
            self.global_index -= 1;
            Ok(())
        }
    }

    pub fn push(&mut self, elem: T) -> usize {
        self.go_to_front_node();
        unsafe {
            self.chunk().push(elem)
        }
    }

    pub fn remove_at(&mut self, global_index: usize) -> Option<T> {
        match self.go_to_node_with_index(global_index) {
            Err(()) => None,
            Ok(()) => {
                self.chunk().remove_at(global_index - self.chunk().node_start_index())
            }
        }
    }

    pub fn go_next_node(&mut self) -> Result<(), ()> {
        match unsafe { self.chunk().next_node() } {
            None => Err(()),
            Some(next) => {
                self.chunk = next;
                self.index = 0;
                self.global_index = next.node_start_index();
                Ok(())
            }
        }
    }
    pub fn go_prev_node(&mut self) -> Result<(), ()> {
        match unsafe { self.chunk().prev_node() } {
            None => Err(()),
            Some(prev) => {
                self.chunk = prev;
                self.index = 0;
                self.global_index = prev.node_start_index();
                Ok(())
            }
        }
    }
    pub fn go_to_front_node(&mut self) {
        while let Ok(()) = self.go_next_node() {
            continue
        }
    }
    pub fn go_to_back_node(&mut self) {
        while let Ok(()) = self.go_prev_node() {
            continue
        }
    }
    pub fn go_to_node_with_index(&mut self, index: usize) -> Result<(),()> {
        while index < self.chunk().node_start_index() {
            self.go_prev_node()?;
        }
        while index >= self.chunk().node_start_index() + self.chunk().node_capacity() {
            self.go_next_node()?;
        }

        Ok(())
    }

    pub fn drain_forward(&mut self) {
        self.go_to_front_node();
        let node_len = self.chunk().node_len();

        self.index = node_len;
        self.item_owed = true;
    }
    pub fn drain_backwards(&mut self) {
        self.go_to_back_node();
        self.index = 0;
        self.item_owed = true;
    }

    pub fn return_current_elem_on_iteration(&mut self, do_return: bool) {
        self.item_owed = do_return;
    }
}

impl<T: 'static> Iterator for ChunkRef<T> {
    type Item = RwLockReadGuard<'static, Option<T>>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.item_owed {
            let result = self.get();
            self.item_owed = result.is_none();
            return result;
        }

        if self.go_next().is_err() {
            return None;
        }
        self.get()
    }
}

impl<T: 'static> DoubleEndedIterator for ChunkRef<T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.item_owed {
            let result = self.get();
            self.item_owed = result.is_none();
            return result;
        }
        
        if self.go_prev().is_err() {
            return None;
        }
        self.get()
    }
}