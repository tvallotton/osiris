use std::mem::size_of;

pub use super::bindings::{self, io_uring_params as Params};

impl Params {
    // Indicates whether the feature for sharing a memory allocation
    // between the two queues is enabled
    pub fn feat_single_allocation(&self) -> bool {
        (self.features & bindings::IORING_FEAT_SINGLE_MMAP) != 0
    }

    pub fn cq_size(&self) -> usize {
        let cqes = self.cq_off.cqes as usize;
        let cq_entries = self.cq_entries as usize;
        cqes + cq_entries * size_of::<bindings::io_uring_cqe>()
    }
    
    pub fn sq_size(&self) -> usize {
        let array = self.sq_off.array as usize;
        let sq_entries = self.sq_entries as usize;
        array + sq_entries * size_of::<u32>()
    }
    // Returns the capacity required to store the SQEs
    pub fn sqes_size(&self) -> usize {
        self.sq_entries as usize * size_of::<bindings::io_uring_sqe>()
    }
}
