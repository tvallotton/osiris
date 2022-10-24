use crate::hasher::NoopHasher;
use std::collections::HashMap;
use std::hash::{Hash};

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
            map: HashMap::with_capacity_and_hasher(capacity, NoopHasher::new()),
        }
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn push_back(&mut self, item: T) {
        self.map.insert(self.last, item);
        self.last = item;
    }

    pub fn pop_front(&mut self) -> Option<T> {
        let (_, first) = self.map.remove_entry(&self.first)?;
        self.first = first;
        Some(first)
    }
}
