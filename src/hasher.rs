use std::hash::{Hasher, BuildHasher};

#[derive(Default)]
pub(crate) struct NoopHasher(u64);

impl NoopHasher {
    pub fn new() -> Self {
        Self::default()
    }
}

impl BuildHasher for NoopHasher {
    type Hasher = NoopHasher;
    #[inline]
    fn build_hasher(&self) -> Self::Hasher {
        NoopHasher(self.0)
    }
}
impl Hasher for NoopHasher {
    fn finish(&self) -> u64 {
        self.0 as u64
    }
    fn write(&mut self, _: &[u8]) {
        unimplemented!()
    }
    fn write_usize(&mut self, i: usize) {
        self.0 = i as u64;
    }
    fn write_u32(&mut self, i: u32) {
        self.0 = i as u64;
    }
    fn write_u64(&mut self, i: u64) {
        self.0 = i as u64;
    }
}
