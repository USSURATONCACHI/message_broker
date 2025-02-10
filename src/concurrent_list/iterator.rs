use super::{Chunk, ChunkReadGuard};


pub struct ChunkIterator<'a, T> {
    chunk: &'a Chunk<T>,
    idx: usize,
}

impl<'a, T> Iterator for ChunkIterator<'a, T> {
    type Item = ChunkReadGuard<'a, T>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.idx == self.chunk.node_capacity() {
            let next = match self.chunk.next_node() {
                None => return None,
                Some(x) => x,
            };

            self.idx -= self.chunk.node_capacity();
            self.chunk = next;
        }

        loop {
            if self.idx == self.chunk.node_len() {
                return None;
            }
            
            match self.chunk.at(self.idx) {
                None => {
                    self.idx += 1;
                },
                Some(elem) => {
                    self.idx += 1;
                    return Some(elem)
                }
            }
        }

    }
}

impl<'a, T> IntoIterator for &'a Chunk<T> {
    type Item = ChunkReadGuard<'a, T>;

    type IntoIter = ChunkIterator<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        ChunkIterator {
            chunk: self,
            idx: 0,
        }
    }
}