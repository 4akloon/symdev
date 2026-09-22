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

macro_rules! narrow {
    ($($t:ty)*) => {$(
        impl Arg for $t {
            fn put<S: Sink + ?Sized>(&self, sink: &mut S) -> fmt::Result {
                sink.put_u32(self.unsigned_abs() as u32, *self < 0)
            }
        }
    )*};
}

narrow!(i8 i16 i32);

macro_rules! narrow_unsigned {
    ($($t:ty)*) => {$(
        impl Arg for $t {
            fn put<S: Sink + ?Sized>(&self, sink: &mut S) -> fmt::Result {
                sink.put_u32(u32::from(*self), false)
            }
        }
    )*};
}

narrow_unsigned!(u8 u16 u32);

/// A 64-bit value whose magnitude fits a `u32` still takes [`Sink::put_u32`]; on the
/// phone `isize` and `usize` always do, so their wide branches are dead code there.
macro_rules! wide_signed {
    ($($t:ty)*) => {$(
        impl Arg for $t {
            fn put<S: Sink + ?Sized>(&self, sink: &mut S) -> fmt::Result {
                match u32::try_from(self.unsigned_abs()) {
                    Ok(magnitude) => sink.put_u32(magnitude, *self < 0),
                    Err(_) => sink.put_i64(*self as i64),
                }
            }
        }
    )*};
}

wide_signed!(i64 isize);

macro_rules! wide_unsigned {
    ($($t:ty)*) => {$(
        impl Arg for $t {
            fn put<S: Sink + ?Sized>(&self, sink: &mut S) -> fmt::Result {
                if let Ok(small) = u32::try_from(*self) {
                    return sink.put_u32(small, false);
                }
                match i64::try_from(*self) {
                    Ok(value) => sink.put_i64(value),
                    Err(_) => sink.put_u64(*self as u64),
                }
            }
        }
    )*};
}

wide_unsigned!(u64 usize);
