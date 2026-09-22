//! The argument types a plain `{}` appends directly: `str`, `String`, `char` and the
//! integers up to 64 bits. Each writes exactly what its `Display` writes with no
//! flags, through the same calls. Everything else — `bool`, floats, 128-bit integers,
//! a user's own `Display` — is not on the list and is formatted by `core::fmt`.

use alloc::string::String;
use core::fmt;

use crate::sink::Sink;

/// A type whose flag-less `Display` a [`Sink`] can reproduce.
pub trait Arg {
    fn put<S: Sink + ?Sized>(&self, sink: &mut S) -> fmt::Result;
}

impl Arg for str {
    fn put<S: Sink + ?Sized>(&self, sink: &mut S) -> fmt::Result {
        sink.put_str(self)
    }
}

impl Arg for String {
    fn put<S: Sink + ?Sized>(&self, sink: &mut S) -> fmt::Result {
        sink.put_str(self)
    }
}

impl Arg for char {
    fn put<S: Sink + ?Sized>(&self, sink: &mut S) -> fmt::Result {
        sink.put_char(*self)
    }
}

/// `Display for &T` and `&mut T` forward to `T`, so these do too.
impl<T: Arg + ?Sized> Arg for &T {
    fn put<S: Sink + ?Sized>(&self, sink: &mut S) -> fmt::Result {
        (**self).put(sink)
    }
}

impl<T: Arg + ?Sized> Arg for &mut T {
    fn put<S: Sink + ?Sized>(&self, sink: &mut S) -> fmt::Result {
        (**self).put(sink)
    }
}

macro_rules! unsigned {
    ($($t:ty)*) => {$(
        impl Arg for $t {
            fn put<S: Sink + ?Sized>(&self, sink: &mut S) -> fmt::Result {
                sink.put_int(*self as u64, false)
            }
        }
    )*};
}

macro_rules! signed {
    ($($t:ty)*) => {$(
        impl Arg for $t {
            fn put<S: Sink + ?Sized>(&self, sink: &mut S) -> fmt::Result {
                sink.put_int(self.unsigned_abs() as u64, *self < 0)
            }
        }
    )*};
}

unsigned!(u8 u16 u32 u64 usize);
signed!(i8 i16 i32 i64 isize);
