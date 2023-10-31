use crate::buf::{IoBuf, IoBufMut};
use std::slice::{from_raw_parts, from_raw_parts_mut};

pub fn slice<B: IoBuf>(b: &B) -> &[u8] {
    unsafe { from_raw_parts(b.stable_ptr(), b.bytes_init()) }
}

pub fn slice_mut<B: IoBufMut>(b: &mut B) -> &mut [u8] {
    unsafe { from_raw_parts_mut(b.stable_mut_ptr(), b.bytes_init()) }
}
