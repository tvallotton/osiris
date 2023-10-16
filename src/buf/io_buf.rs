use crate::buf::Slice;

use std::ops;
use std::rc::Rc;

/// An `io-uring` compatible buffer.
///
/// The `IoBuf` trait is implemented by buffer types that can be passed to
/// io-uring operations. Users will not need to use this trait directly, except
/// for the [`slice`] method.
///
/// # Slicing
///
/// Because buffers are passed by ownership to the runtime, Rust's slice API
/// (`&buf[..]`) cannot be used. Instead, `osiris` provides an owned slice
/// API: [`slice()`]. The method takes ownership fo the buffer and returns a
/// `Slice<Self>` type that tracks the requested offset.
///
/// # Safety
///
/// Buffers passed to `io-uring` operations must reference a stable memory
/// region. While the runtime holds ownership to a buffer, the pointer returned
/// by `stable_ptr` must remain valid even if the `IoBuf` value is moved.
///
/// [`slice()`]: IoBuf::slice
pub unsafe trait IoBuf: Unpin + 'static {
    /// Returns a raw pointer to the vector’s buffer.
    ///
    /// This method is to be used by the `osiris` runtime and it is not
    /// expected for users to call it directly.
    ///
    /// The implementation must ensure that, while the `osiris` runtime
    /// owns the value, the pointer returned by `stable_ptr` **does not**
    /// change.
    fn stable_ptr(&self) -> *const u8;

    /// Number of initialized bytes.
    ///
    /// This method is to be used by the `osiris` runtime and it is not
    /// expected for users to call it directly.
    ///
    /// For `Vec`, this is identical to `len()`.
    fn bytes_init(&self) -> usize;

    /// Total size of the buffer, including uninitialized memory, if any.
    ///
    /// This method is to be used by the `osiris` runtime and it is not
    /// expected for users to call it directly.
    ///
    /// For `Vec`, this is identical to `capacity()`.
    fn bytes_total(&self) -> usize;

    /// Returns a view of the buffer with the specified range.
    ///
    /// This method is similar to Rust's slicing (`&buf[..]`), but takes
    /// ownership of the buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use osiris::buf::IoBuf;
    ///
    /// let buf = b"hello world".to_vec();
    /// buf.slice(5..10);
    /// ```
    fn slice(self, range: impl ops::RangeBounds<usize>) -> Slice<Self>
    where
        Self: Sized,
    {
        use core::ops::Bound;

        let begin = match range.start_bound() {
            Bound::Included(&n) => n,
            Bound::Excluded(&n) => n + 1,
            Bound::Unbounded => 0,
        };

        assert!(begin < self.bytes_total());

        let end = match range.end_bound() {
            Bound::Included(&n) => n.checked_add(1).expect("out of range"),
            Bound::Excluded(&n) => n,
            Bound::Unbounded => self.bytes_total(),
        };

        assert!(end <= self.bytes_total());
        assert!(begin <= self.bytes_init());

        Slice::new(self, begin, end)
    }
}
// Safety: Vec<u8> allocates memory which is stable.
unsafe impl IoBuf for Vec<u8> {
    fn stable_ptr(&self) -> *const u8 {
        self.as_ptr()
    }

    fn bytes_init(&self) -> usize {
        self.len()
    }

    fn bytes_total(&self) -> usize {
        self.capacity()
    }
}
// Safety: static references are stable
unsafe impl IoBuf for &'static [u8] {
    fn stable_ptr(&self) -> *const u8 {
        self.as_ptr()
    }

    fn bytes_init(&self) -> usize {
        <[u8]>::len(self)
    }

    fn bytes_total(&self) -> usize {
        self.bytes_init()
    }
}

unsafe impl<const N: usize> IoBuf for &'static [u8; N] {
    fn stable_ptr(&self) -> *const u8 {
        self.as_ptr()
    }
    fn bytes_init(&self) -> usize {
        N
    }
    fn bytes_total(&self) -> usize {
        N
    }
}

// Safety: static references are stable
unsafe impl IoBuf for &'static str {
    fn stable_ptr(&self) -> *const u8 {
        self.as_ptr()
    }

    fn bytes_init(&self) -> usize {
        <str>::len(self)
    }

    fn bytes_total(&self) -> usize {
        self.bytes_init()
    }
}
// Safety: Rc are stable pointers
unsafe impl IoBuf for Rc<[u8]> {
    fn stable_ptr(&self) -> *const u8 {
        self.as_ptr()
    }

    fn bytes_init(&self) -> usize {
        self.len()
    }

    fn bytes_total(&self) -> usize {
        self.len()
    }
}

// Safety: Rc are stable pointers
unsafe impl<const N: usize> IoBuf for Rc<[u8; N]> {
    fn stable_ptr(&self) -> *const u8 {
        self.as_ptr()
    }

    fn bytes_init(&self) -> usize {
        self.len()
    }

    fn bytes_total(&self) -> usize {
        self.len()
    }
}

// Safety: Rc are stable pointers
unsafe impl IoBuf for Rc<str> {
    fn stable_ptr(&self) -> *const u8 {
        self.as_ptr()
    }

    fn bytes_init(&self) -> usize {
        self.len()
    }

    fn bytes_total(&self) -> usize {
        self.len()
    }
}
// Safety: guaranteed by T: IoBuf
unsafe impl<T: IoBuf> IoBuf for Rc<T> {
    fn stable_ptr(&self) -> *const u8 {
        T::stable_ptr(self)
    }
    fn bytes_init(&self) -> usize {
        T::bytes_init(self)
    }
    fn bytes_total(&self) -> usize {
        T::bytes_total(self)
    }
}
// Safety: Boxes are stable pointers
unsafe impl IoBuf for Box<str> {
    fn stable_ptr(&self) -> *const u8 {
        self.as_ptr()
    }
    fn bytes_init(&self) -> usize {
        self.len()
    }
    fn bytes_total(&self) -> usize {
        self.len()
    }
}
// Safety: Boxes are stable pointers
unsafe impl IoBuf for Box<[u8]> {
    fn stable_ptr(&self) -> *const u8 {
        self.as_ptr()
    }
    fn bytes_init(&self) -> usize {
        self.len()
    }
    fn bytes_total(&self) -> usize {
        self.len()
    }
}
// Safety: String is a stable pointer
unsafe impl IoBuf for String {
    fn stable_ptr(&self) -> *const u8 {
        self.as_ptr()
    }
    fn bytes_init(&self) -> usize {
        self.len()
    }
    fn bytes_total(&self) -> usize {
        self.capacity()
    }
}

// Safety: Boxes are stable pointers
unsafe impl<const N: usize> IoBuf for Box<[u8; N]> {
    fn stable_ptr(&self) -> *const u8 {
        self.as_ptr()
    }
    fn bytes_init(&self) -> usize {
        self.len()
    }
    fn bytes_total(&self) -> usize {
        self.len()
    }
}
