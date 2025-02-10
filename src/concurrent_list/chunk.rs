use std::{mem::forget, ops::Deref, sync::{atomic::{AtomicPtr, AtomicUsize, Ordering}, RwLock}};

use super::{ChunkIterator, ChunkReadGuard};



pub static TOTAL_READS: AtomicUsize = AtomicUsize::new(0);
pub static READ_LOCKS: AtomicUsize = AtomicUsize::new(0);
pub static TOTAL_APPENDS: AtomicUsize = AtomicUsize::new(0);
pub static APPEND_LOCKS: AtomicUsize = AtomicUsize::new(0);
pub static APPEND_MISSES: AtomicUsize = AtomicUsize::new(0);
pub static TOTAL_ELEMENTS_WRITTEN: AtomicUsize = AtomicUsize::new(0);

pub struct Chunk<T> {
    data: Box<[RwLock<Option<T>>]>,
    length: AtomicUsize,
    prev: AtomicPtr<Chunk<T>>,
    next: AtomicPtr<Chunk<T>>,
}




impl<T> Chunk<T> {
    pub fn iter<'a>(&'a self) -> ChunkIterator<'a, T> {
        self.into_iter()
    }

    pub fn new(prev: Option<&Chunk<T>>, cap: usize) -> Self {
        Self {
            data: (0..cap).map(|_| RwLock::new(None)).collect(),
            length: AtomicUsize::new(0),

            prev: match prev {
                None => AtomicPtr::new(std::ptr::null::<Chunk<T>>() as *mut _),
                Some(prev) => AtomicPtr::new(prev as *const _  as *mut _)
            },
            next: AtomicPtr::new(std::ptr::null::<Chunk<T>>() as *mut _),
        }
    }

    pub fn node_len(&self) -> usize {
        return self.length.load(Ordering::Relaxed);
    }

    pub fn total_len(&self) -> usize {
        match self.next_node() {
            None => self.node_len(),
            Some(next) => self.node_len() + next.total_len(),
        }
    }

    pub fn nodes_count(&self) -> usize {
        match self.next_node() {
            None => 1,
            Some(next) => 1 + next.nodes_count(),
        }
    }

    pub fn node_capacity(&self) -> usize {
        return self.data.len();
    }

    pub fn next_node(&self) -> Option<&Self> {
        let ptr = self.next.load(Ordering::Relaxed);
        if ptr.is_null() {
            None
        } else {
            unsafe { Some(ptr.as_ref().unwrap()) }
        }
    }
    pub fn prev_node(&self) -> Option<&Self> {
        let ptr = self.prev.load(Ordering::Relaxed);
        if ptr.is_null() {
            None
        } else {
            unsafe { Some(ptr.as_ref().unwrap()) }
        }
    }

    pub fn at<'a>(&'a self, mut index: usize) -> Option<ChunkReadGuard<'a, T>> {
        let mut current_node = self;

        // Jump into node with that index
        while index > current_node.data.len() {
            index -= current_node.data.len();

            current_node = match self.next_node() {
                None => return None,
                Some(x) => x,
            }
        }

        // If index does not exist in such node
        if index >= current_node.node_len() {
            return None;
        }

        // If element was deleted (overwritten with None) - return None
        let lock;

        loop {
            match current_node.data[index].try_read() {
                Ok(l) => {
                    lock = l;
                    break;
                },
                Err(e) => match e {
                    std::sync::TryLockError::Poisoned(poison_error) => 
                        panic!("Poisoned RwLock: {poison_error}"),

                    std::sync::TryLockError::WouldBlock => {
                        READ_LOCKS.fetch_add(1, Ordering::Relaxed);
                    },
                },
            }
        }
        TOTAL_READS.fetch_add(1, Ordering::Relaxed);

        if lock.deref().is_none() {
            None
        } else {
            // Finally
            Some(ChunkReadGuard::from_rwlock_read(lock))
        }
    }

    pub fn push(&self, val: T) -> usize {
        let mut current_node = self;
        let write_index: usize;

        // Allocate an index
        loop {
            let len = current_node.node_len();

            // Allocate a node if it does not exist
            if len >= current_node.node_capacity() {
                let next_ptr = current_node.next.load(Ordering::Relaxed);

                if next_ptr.is_null() {
                    // Allocate next node
                    let mut new_node: Box<Self> = Box::new(Self::new(Some(current_node), current_node.node_capacity()));
                    let new_ptr = new_node.as_mut() as *mut Self;

                    // Try to atomically swap
                    let result = current_node.next
                        .compare_exchange_weak(next_ptr, new_ptr, Ordering::Relaxed, Ordering::Relaxed);
                    
                    match result {
                        Ok(_) => {
                            forget(new_node);
                            current_node = current_node.next_node().unwrap();
                        }
                        Err(_) => {
                            APPEND_MISSES.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                } else {
                    current_node = unsafe { next_ptr.as_ref().unwrap() };
                }
                continue;
            }
            
            // Aquire the last non-taken index
            let result = current_node.length
                .compare_exchange_weak(len, len + 1, Ordering::Relaxed, Ordering::Relaxed);

            match result {
                Err(_) => continue,
                Ok(_) => {
                    write_index = len;
                    break;
                },
            }
        }

        // Aquire a read guard and write into it.
        let mut lock: std::sync::RwLockWriteGuard<'_, Option<T>>;
        loop {
            match current_node.data[write_index].try_write() {
                Ok(l) => {
                    lock = l;
                    break;
                },
                Err(e) => match e {
                    std::sync::TryLockError::Poisoned(poison_error) => 
                        panic!("Poisoned RwLock: {poison_error}"),
                        
                    std::sync::TryLockError::WouldBlock => {
                        APPEND_LOCKS.fetch_add(1, Ordering::Relaxed);
                    },
                },
            }
        }
        TOTAL_APPENDS.fetch_add(1, Ordering::Relaxed);

        *lock = Some(val);

        write_index
    }
}
