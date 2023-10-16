use osiris::buf::{IoBuf, IoBufMut};
use std::rc::Rc;

const ARRAY: [u8; 10] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
const TEXT: &'static str = "hello world";

#[osiris::test]
async fn io_buf_box_u8() {
    let b = Box::new(ARRAY);
    assert!(std::ptr::eq(b.as_ptr(), b.stable_ptr()));
    assert_eq!(b.bytes_init(), ARRAY.len());
    assert_eq!(b.bytes_total(), ARRAY.len());
}

#[osiris::test]
async fn io_buf_box_u8_const() {
    let b: Box<[u8; 10]> = Box::new(ARRAY);
    assert!(std::ptr::eq(b.as_ptr(), b.stable_ptr()));
    assert_eq!(b.bytes_init(), ARRAY.len());
    assert_eq!(b.bytes_total(), ARRAY.len());
}

#[osiris::test]
async fn io_buf_box_str() {
    let b = Box::new(TEXT);
    assert!(std::ptr::eq(b.as_ptr(), b.stable_ptr()));
    assert_eq!(b.bytes_init(), TEXT.len());
    assert_eq!(b.bytes_total(), TEXT.len());
}

#[osiris::test]
async fn io_buf_vec_u8() {
    let mut b = Vec::from(ARRAY);
    b.reserve_exact(1);
    assert!(std::ptr::eq(b.as_ptr().cast(), b.stable_ptr()));
    assert_eq!(b.bytes_init(), ARRAY.len());
    assert_eq!(b.bytes_total(), ARRAY.len() + 1);
}

#[osiris::test]
async fn io_buf_ref_u8() {
    let b = &ARRAY;
    assert!(std::ptr::eq(b.as_ptr().cast(), b.stable_ptr()));
    assert_eq!(b.bytes_init(), ARRAY.len());
    assert_eq!(b.bytes_total(), ARRAY.len());
}

#[osiris::test]
async fn io_buf_ref_str() {
    let b = TEXT;
    assert!(std::ptr::eq(b.as_ptr().cast(), b.stable_ptr()));
    assert_eq!(b.bytes_init(), TEXT.len());
    assert_eq!(b.bytes_total(), TEXT.len());
}

#[osiris::test]
async fn io_buf_string() {
    let mut b = String::from(TEXT);
    b.reserve_exact(1);
    assert!(std::ptr::eq(b.as_ptr().cast(), b.stable_ptr()));
    assert_eq!(b.bytes_init(), TEXT.len());
    assert_eq!(b.bytes_total(), TEXT.len() + 1);
}

#[osiris::test]
async fn io_buf_rc_str() {
    let b: Rc<str> = Rc::from(TEXT);

    assert!(std::ptr::eq(b.as_ptr().cast(), b.stable_ptr()));
    assert_eq!(b.bytes_init(), TEXT.len());
    assert_eq!(b.bytes_total(), TEXT.len());
}

#[osiris::test]
async fn io_buf_rc_u8() {
    let b: Rc<[u8]> = Rc::from(ARRAY);
    assert!(std::ptr::eq(b.as_ptr(), b.stable_ptr()));
    assert_eq!(b.bytes_init(), ARRAY.len());
    assert_eq!(b.bytes_total(), ARRAY.len());
}

#[osiris::test]
async fn io_buf_rc_u8_const() {
    let b: Rc<[u8; 10]> = Rc::from(ARRAY);
    assert!(std::ptr::eq(b.as_ptr(), b.stable_ptr()));
    assert_eq!(b.bytes_init(), ARRAY.len());
    assert_eq!(b.bytes_total(), ARRAY.len());
}

#[osiris::test]
async fn io_buf_slice() {
    let b = Vec::from(ARRAY);
    let slice = b.slice(1..);
    slice.stable_ptr();
    assert_eq!(slice.bytes_init(), ARRAY.len() - 1);
    assert_eq!(slice.bytes_total(), ARRAY.len() - 1);
    assert_eq!(slice.into_inner(), ARRAY);
}
