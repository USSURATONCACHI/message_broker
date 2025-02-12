pub struct ReverseIterator<T: DoubleEndedIterator> {
    t: T
}

impl<T: DoubleEndedIterator> ReverseIterator<T> {
    pub fn new(t: T) -> Self { t.into() }

    pub fn into_inner(self) -> T { self.t }

    pub fn inner(&self) -> &T { &self.t }
    pub fn inner_mut(&mut self) -> &mut T { &mut self.t }
}

impl<T: Default + DoubleEndedIterator> Default for ReverseIterator<T> {
    fn default() -> Self {
        Self { t: Default::default() }
    }
}

impl<T: DoubleEndedIterator> From<T> for ReverseIterator<T> {
    fn from(t: T) -> Self {
        Self { t }
    }
}

impl<T: DoubleEndedIterator> Iterator for ReverseIterator<T> {
    type Item = <T as Iterator>::Item;

    fn next(&mut self) -> Option<Self::Item> {
        self.t.next_back()
    }
}