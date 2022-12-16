use super::bindings::{self, IoUringParams};

impl IoUringParams {
    pub fn feat_single_map(&self) -> bool {
        (self.features & bindings::IORING_FEAT_SINGLE_MMAP) != 0
    }
}
