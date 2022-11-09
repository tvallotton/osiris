use crate::hasher::NoopHasher;
use std::collections::HashMap;

#[derive(Debug)]
pub(crate) struct UniqueQueue {
    first: usize,
    last: usize,
    map: HashMap<usize, usize, NoopHasher>,
}

impl UniqueQueue {
    pub fn with_capacity(capacity: usize) -> Self {
        UniqueQueue {
            first: usize::MAX,
            last: usize::MAX,
            map: HashMap::with_capacity_and_hasher(capacity, NoopHasher::new()),
        }
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn push_back(&mut self, item: usize) {
        self.map.insert(self.last, item);
        self.last = item;
    }

    pub fn pop_front(&mut self) -> Option<usize> {
        let (_, first) = self.map.remove_entry(&self.first)?;
        self.first = first;
        Some(first)
    }
}

#[test]
fn smoke_test() {
    let mut queue = UniqueQueue::with_capacity(8);

    queue.push_back(0);
    queue.push_back(1);
    assert_eq!(queue.pop_front(), Some(0));
    assert_eq!(queue.pop_front(), Some(1));
}
