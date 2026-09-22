//! What a case shows of a value it was decided on, written without `core::fmt`.
use alloc::string::String;
// What the fast `write!` falls back to for a piece not on its list; nothing here has one.
use alloc::vec::Vec;
use core::fmt::Write as _;

use symbian_core::SymbianError;

use crate::write;

/// A value a report can show as text: the error [`super::Report::checked`] records,
/// or an outcome in a [`crate::detail!`].
///
/// The text is `{:?}`'s for everything here, except where noted, so a report reads as
/// it did when it formatted through `Debug`; what differs is that none of it links
/// `core::fmt`. A Symbian error is its `e32err.h` name and its `TInt`,
/// `KErrNotFound (-1)`, which is what identifies it.
pub trait Evidence {
    /// Appends the text to `out`.
    fn show(&self, out: &mut String);

    /// The text on its own.
    fn shown(&self) -> String {
        let mut out = String::new();
        self.show(&mut out);
        out
    }
}

/// `{:#x}` of a 32-bit value (an address or a handle on the phone): `0x1f`.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Hex(pub u32);

impl Evidence for Hex {
    fn show(&self, out: &mut String) {
        out.push_str("0x");
        super::json::push_hex(out, self.0, 1);
    }
}

/// `KErrNotFound (-1)`, as its `Debug`.
impl Evidence for SymbianError {
    fn show(&self, out: &mut String) {
        let _ = write!(out, "{} ({})", self.kind().name(), self.code());
    }
}

/// `KErrNotFound (-1)`: the Symbian error it carries. Its `Debug` also prefixes the
/// `std` kind (`NotFound (KErrNotFound (-1))`), which follows from the code and would
/// cost a table of names.
#[cfg(not(feature = "std"))]
impl Evidence for crate::io::Error {
    fn show(&self, out: &mut String) {
        self.as_symbian().show(out);
    }
}

/// Its `Display`, which is also its `Debug`: which of the three failures it was.
#[cfg(not(feature = "std"))]
impl Evidence for crate::thread::AccessError {
    fn show(&self, out: &mut String) {
        out.push_str(self.message());
    }
}

/// `SystemTimeError(<n> us)`: how far the wrong way round the two times were. Its
/// `Debug` writes the `Duration` as `Duration`'s `Debug` does (`1.5s`); microseconds
/// are the unit the clock has.
#[cfg(not(feature = "std"))]
impl Evidence for crate::time::SystemTimeError {
    fn show(&self, out: &mut String) {
        let apart = self.duration();
        // In `u64`: `Duration::as_micros` is a `u128`, whose arithmetic is library code
        // on ARM.
        let micros = apart
            .as_secs()
            .saturating_mul(1_000_000)
            .saturating_add(u64::from(apart.subsec_micros()));
        let _ = write!(out, "SystemTimeError({} us)", micros);
    }
}

/// `std::io::Error` under the `std` feature, where `core::fmt` is linked by `std`
/// itself: its `Debug`, unchanged.
#[cfg(feature = "std")]
impl Evidence for std::io::Error {
    fn show(&self, out: &mut String) {
        let _ = core::fmt::Write::write_fmt(out, format_args!("{self:?}"));
    }
}

impl Evidence for () {
    fn show(&self, out: &mut String) {
        out.push_str("()");
    }
}

impl Evidence for bool {
    fn show(&self, out: &mut String) {
        out.push_str(if *self { "true" } else { "false" });
    }
}

macro_rules! integers {
    ($($t:ty)*) => {$(
        impl Evidence for $t {
            fn show(&self, out: &mut String) {
                let _ = write!(out, "{}", self);
            }
        }
    )*};
}

integers!(i8 i16 i32 i64 isize u8 u16 u32 u64 usize);

/// Quoted, as `Debug` quotes it; the text itself is not escaped (the report's JSON
/// escapes it on the way into the file).
impl Evidence for str {
    fn show(&self, out: &mut String) {
        out.push('"');
        out.push_str(self);
        out.push('"');
    }
}

impl Evidence for String {
    fn show(&self, out: &mut String) {
        self.as_str().show(out);
    }
}

impl<T: Evidence + ?Sized> Evidence for &T {
    fn show(&self, out: &mut String) {
        (**self).show(out);
    }
}

impl<T: Evidence, E: Evidence> Evidence for Result<T, E> {
    fn show(&self, out: &mut String) {
        match self {
            Ok(value) => wrapped(out, "Ok(", value),
            Err(error) => wrapped(out, "Err(", error),
        }
    }
}

impl<T: Evidence> Evidence for Option<T> {
    fn show(&self, out: &mut String) {
        match self {
            Some(value) => wrapped(out, "Some(", value),
            None => out.push_str("None"),
        }
    }
}

impl<L: Evidence, R: Evidence> Evidence for symbian_async::Either<L, R> {
    fn show(&self, out: &mut String) {
        match self {
            Self::Left(value) => wrapped(out, "Left(", value),
            Self::Right(value) => wrapped(out, "Right(", value),
        }
    }
}

/// `[1, 2]`.
impl<T: Evidence> Evidence for [T] {
    fn show(&self, out: &mut String) {
        out.push('[');
        for (i, item) in self.iter().enumerate() {
            if i > 0 {
                out.push_str(", ");
            }
            item.show(out);
        }
        out.push(']');
    }
}

impl<T: Evidence> Evidence for Vec<T> {
    fn show(&self, out: &mut String) {
        self.as_slice().show(out);
    }
}

/// `Some(` + the value + `)`.
fn wrapped(out: &mut String, open: &str, value: &dyn Evidence) {
    out.push_str(open);
    value.show(out);
    out.push(')');
}

/// A pair, as a tuple's `Debug`: `(1, Ok(()))`.
impl<A: Evidence, B: Evidence> Evidence for (A, B) {
    fn show(&self, out: &mut String) {
        out.push('(');
        self.0.show(out);
        out.push_str(", ");
        self.1.show(out);
        out.push(')');
    }
}
