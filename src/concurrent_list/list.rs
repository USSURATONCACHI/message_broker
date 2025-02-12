use std::sync::Arc;
use super::ChunkRef;
use super::inner::ConcurrentListInner;

use serde::ser::SerializeSeq;
use serde::{Serialize, Serializer, Deserialize, Deserializer};
use serde::de::{Visitor, SeqAccess};

/// [`ConcurrentList`] supports any amount of concurrent/parallel readers and writers.
/// List is accessed via `Arc<Self>::reference()` which is a [`ChunkRef<T>`].
/// 
/// [`ChunkRef<T>`] can be used to `.push(T)` and `.remove_at(index)` safely.
/// [`ChunkRef<T>`] is also a double-sided iterator. And can be manually moved in the list.
/// [`ChunkRef<T>`] is safe to clone. Cloning it is the recommended way to share access to the list. 
/// 
/// [`ConcurrentList`] is guaranteed to be almost* lock-free.
///  
/// *Almost: in practice, there will be zero locks.
/// But if you have a lot of readers that will try to access new elements in the time they are written
/// you may have less than 0.1% of locks. In these 0.1% cases - readers/writers will spin on the RwLock
/// a few times. Effect is essentially negligible.
/// 
/// ```
/// use std::sync::Arc;
/// use std::sync::RwLockReadGuard;
/// use std::ops::Deref;
/// use broker::concurrent_list::{ConcurrentList, ChunkRef};
/// 
/// fn main() {
///     // 256 chunk size is fairly optimal. More chunk size = more performace but less efficient allocations. 
///     let list = ConcurrentList::<String>::new(256); 
///     let mut threads = vec![];
/// 
///     for _ in 0..10 { // Any amount of parallel accesses
///         let handle: ChunkRef<String> = list.reference();
///         let thread = std::thread::spawn(move || do_something(handle));
///         threads.push(thread);
///     }
/// 
///     threads.into_iter().for_each(|thread| thread.join().unwrap());
/// }
/// 
/// fn do_something(mut handle: ChunkRef<String>) {
///     handle.push("An element".to_string());
/// 
///     handle.drain_backwards(); // Go to the very first element.
/// 
///     for elem in handle {
///         let elem: RwLockReadGuard<'_, Option<String>> = elem;
///         match elem.deref() {
///             None => continue,
///             Some(string_ref) => println!("{string_ref}"),
///         };
///     }
/// }
/// ```

#[derive(Default)]
pub struct ConcurrentList<T> {
    arc: Arc<ConcurrentListInner<T>>,
}

unsafe impl<T> Sync for ConcurrentList<T> {}
unsafe impl<T> Send for ConcurrentList<T> {}

impl<T> ConcurrentList<T> {
    pub fn new(cap: usize) -> Self {
        Self { arc: Arc::new(ConcurrentListInner::new(cap)) }
    }

    pub fn reference(&self) -> ChunkRef<T> {
        unsafe {
            ChunkRef::new_at(
                self.arc.clone(), 
                self.arc.ownership_chunk.as_ref().unwrap(), 
                0
            ).unwrap()
        }
    }

    pub fn len(&self) -> usize { self.arc.len() }
    pub fn nodes_count(&self) -> usize { self.arc.nodes_count() }
}

impl<T: 'static + Serialize> Serialize for ConcurrentList<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(None)?;
        let mut handle = self.reference();
        handle.drain_backwards(); // Go to the very first element
        
        // Iterate through each element, serialize `Some` values
        for elem in handle {
            let elem_guard = elem;
            if let Some(value) = &*elem_guard {
                seq.serialize_element(value)?;
            }
        }
        seq.end()
    }
}

impl<'de, T: 'static + Deserialize<'de>> Deserialize<'de> for ConcurrentList<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // Visitor to handle deserialization
        struct ConcurrentListVisitor<T> {
            _phantom: std::marker::PhantomData<T>,
        }

        impl<'de, T: 'static + Deserialize<'de>> Visitor<'de> for ConcurrentListVisitor<T> {
            type Value = ConcurrentList<T>;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a sequence of elements")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                // Create a new list with default chunk size
                let list = ConcurrentList::new(256);
                let mut handle = list.reference();

                // Deserialize each element and push into the list
                while let Some(elem) = seq.next_element()? {
                    handle.push(elem);
                }

                Ok(list)
            }
        }

        // Use the visitor to deserialize
        deserializer.deserialize_seq(ConcurrentListVisitor {
            _phantom: std::marker::PhantomData,
        })
    }
}