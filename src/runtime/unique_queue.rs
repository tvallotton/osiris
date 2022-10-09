use std::{
    collections::HashMap,
    hash::{BuildHasher, Hash, Hasher},
};

pub(crate) struct NoopHasher(pub usize);
impl Hasher for NoopHasher {
    fn finish(&self) -> u64 {
        self.0 as u64
    }
    fn write(&mut self, _: &[u8]) {
        todo!()
    }
    fn write_usize(&mut self, i: usize) {
        self.0 = i;
    }
}

impl BuildHasher for NoopHasher {
    type Hasher = NoopHasher;
    #[inline]
    fn build_hasher(&self) -> Self::Hasher {
        NoopHasher(self.0)
    }
}

#[derive(Debug)]
pub(crate) struct UniqueQueue<T> {
    first: T,
    last: T,
    map: HashMap<T, T, NoopHasher>,
}

impl<T: Eq + Copy + Hash + Default + Ord> UniqueQueue<T> {
    pub fn with_capacity(capacity: usize) -> Self {
        UniqueQueue {
            first: T::default(),
            last: T::default(),
            map: HashMap::with_capacity_and_hasher(capacity, NoopHasher(0)),
        }
    }
    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn push_back(&mut self, item: T) {
        self.map.insert(self.last, item);
        self.last = item;
    }
    // #[inline(never)]
    pub fn pop_front(&mut self) -> Option<T> {
        let (_, first) = self.map.remove_entry(&self.first)?;
        self.first = first;
        Some(first)
    }
}
