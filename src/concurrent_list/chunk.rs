use std::{ops::{DerefMut}, ptr::null_mut, sync::{atomic::{AtomicPtr, AtomicUsize, Ordering}, RwLock, RwLockReadGuard, RwLockWriteGuard}};

pub static TOTAL_READS: AtomicUsize = AtomicUsize::new(0);
pub static READ_LOCKS: AtomicUsize = AtomicUsize::new(0);
pub static TOTAL_APPENDS: AtomicUsize = AtomicUsize::new(0);
pub static APPEND_LOCKS: AtomicUsize = AtomicUsize::new(0);
pub static APPEND_MISSES: AtomicUsize = AtomicUsize::new(0);
pub static TOTAL_ELEMENTS_WRITTEN: AtomicUsize = AtomicUsize::new(0);

/// A struct with no clear ownership. Only module-private usage is allowed
pub struct Chunk<T> {
    start_index: usize,
    data: Box<[RwLock<Option<T>>]>,
    length: AtomicUsize,
    prev: AtomicPtr<Chunk<T>>, // AtomicArcs would not work here
    next: AtomicPtr<Chunk<T>>,
}

unsafe impl<T> Sync for Chunk<T> {}
unsafe impl<T> Send for Chunk<T> {}


impl<T> Chunk<T> {
    pub fn drop_all_links(&mut self) {
        let next = unsafe { self.next.load(Ordering::Relaxed).as_mut() };
        let prev = unsafe { self.prev.load(Ordering::Relaxed).as_mut() };

        if let Some(next) = next {
            next.prev.store(null_mut(), Ordering::Relaxed);
            unsafe {
                Box::from_raw(next as *mut Chunk<T>).drop_all_links();
            }
        }
        if let Some(prev) = prev {
            prev.next.store(null_mut(), Ordering::Relaxed);
            unsafe {
                Box::from_raw(prev as *mut Chunk<T>).drop_all_links();
            }
        }
    }
}

impl<T> Chunk<T> {
    pub fn node_len(&self) -> usize {
        self.length.load(Ordering::Relaxed)
    }
    pub fn node_capacity(&self) -> usize {
        return self.data.len();
    }
    pub fn node_start_index(&self) -> usize {
        self.start_index
    }

    pub unsafe fn front_elems_count(&self) -> usize {
        match self.next_node() {
            None => self.node_len(),
            Some(next) => self.node_len() + next.front_elems_count(),
        }
    }
    pub unsafe fn front_nodes_count(&self) -> usize {
        match self.next_node() {
            None => 1,
            Some(next) => 1 + next.front_nodes_count(),
        }
    }

    pub unsafe fn back_elems_count(&self) -> usize {
        match self.prev_node() {
            None => 0,
            Some(prev) => prev.node_len() + prev.back_elems_count(),
        }
    }
    pub unsafe fn back_nodes_count(&self) -> usize {
        match self.prev_node() {
            None => 0,
            Some(prev) => 1 + prev.back_nodes_count(),
        }
    }


    pub unsafe fn next_node(&self) -> Option<&Self> {
        let ptr = self.next.load(Ordering::Relaxed);
        if ptr.is_null() {
            None
        } else {
            unsafe { Some(ptr.as_ref().unwrap()) }
        }
    }
    pub unsafe fn prev_node(&self) -> Option<&Self> {
        let ptr = self.prev.load(Ordering::Relaxed);
        if ptr.is_null() {
            None
        } else {
            unsafe { Some(ptr.as_ref().unwrap()) }
        }
    }
}

impl<T> Chunk<T> {
    pub fn new(prev: Option<&Chunk<T>>, cap: usize) -> Self {
        assert!(cap > 0);

        Self {
            start_index: match prev {
                None => 0,
                Some(prev) => prev.start_index + prev.data.len(),
            },

            data: (0..cap).map(|_| RwLock::new(None)).collect(),
            length: 0.into(),

            prev: match prev {
                None => AtomicPtr::new(std::ptr::null::<Chunk<T>>() as *mut _),
                Some(prev) => AtomicPtr::new(prev as *const _  as *mut _)
            },
            next: AtomicPtr::new(std::ptr::null::<Chunk<T>>() as *mut _),
        }
    }

    pub fn at<'a>(&'a self, rel_index: usize) -> Option<RwLockReadGuard<'a, Option<T>>> {
        if rel_index >= self.node_len() {
            return None;
        }

        let lock;

        loop {
            match self.data[rel_index].try_read() {
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

        Some(lock)
    }

    pub fn at_mut<'a>(&'a self, rel_index: usize) -> Option<RwLockWriteGuard<'a, Option<T>>> {
        if rel_index >= self.node_len() {
            return None;
        }

        // If element was deleted (overwritten with None) - return None
        let lock;

        loop {
            match self.data[rel_index].try_write() {
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
        Some(lock)
    }

    pub unsafe fn push(&self, val: T) -> usize {
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
                    let new_node: Box<Self> = Box::new(Self::new(Some(current_node), current_node.node_capacity()));
                    let new_ptr = Box::into_raw(new_node);

                    // Try to atomically swap
                    let result = current_node.next
                        .compare_exchange_weak(next_ptr, new_ptr, Ordering::Relaxed, Ordering::Relaxed);
                    
                    match result {
                        Ok(_) => {
                            current_node = current_node.next_node().unwrap();
                        }
                        Err(_) => {
                            drop(Box::from_raw(new_ptr));
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

        write_index + current_node.start_index
    }

    pub fn remove_at(&self, rel_index: usize) -> Option<T> {
        match self.at_mut(rel_index) {
            None => None,
            Some(mut guard) => {
                let mut result = None;
                std::mem::swap(RwLockWriteGuard::deref_mut(&mut guard), &mut result);
                result
            }
        }
    }
}
