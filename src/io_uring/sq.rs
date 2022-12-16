pub struct Sq {
    pub head: *mut u32,
    pub tail: *mut u32,
    pub ring_mask: *mut u32,
    pub ring_entries: *mut u32,
    pub flags: *mut u32,
    pub array: *mut u32,
}
