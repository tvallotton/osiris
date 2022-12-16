use super::bindings;

pub struct Cq {
    pub head: *mut u32,
    pub tail: *mut u32,
    pub ring_mask: *mut u32,
    pub ring_entries: *mut u32,
    pub cqes: *mut bindings::io_uring_cqe,
}
