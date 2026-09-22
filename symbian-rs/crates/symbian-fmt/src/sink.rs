//! Where a fast piece goes: a destination with a native append, or any `fmt::Write`.

use core::fmt;

use crate::decimal::Decimal;

/// A destination that appends text and numbers itself, without `core::fmt`.
///
/// It is implemented for `symbian_core::Buf16`, whose numbers are euser's
/// `TDes16::AppendNum` in ROM. Every method must leave the destination exactly as the
/// same `write!` would have — including after a failure, which is where it is easy
/// to differ (see [`Sink::put_int`]).
pub trait Sink {
    /// `write_str(s)`.
    fn put_str(&mut self, s: &str) -> fmt::Result;

    /// `write_char(c)`.
    fn put_char(&mut self, c: char) -> fmt::Result;

    /// An integer's `Display` with no flags, for an integer whose magnitude fits a
    /// `u32` (every one up to 32 bits wide). `core` writes the sign with
    /// `write_char('-')` and then the digits with one `write_str`, so a destination
    /// with room for the sign and not the digits ends up holding the `-`.
    fn put_u32(&mut self, magnitude: u32, negative: bool) -> fmt::Result;

    /// As [`Sink::put_u32`], for an `i64` whose magnitude does not fit a `u32`. The
    /// integer widths are separate methods so that a program that formats nothing
    /// wider than 32 bits never links the 64-bit conversion.
    fn put_i64(&mut self, value: i64) -> fmt::Result;

    /// A `u64` above `i64::MAX`, which [`Sink::put_i64`] cannot take.
    fn put_u64(&mut self, value: u64) -> fmt::Result;
}

/// Any `fmt::Write`, driven through the calls `core::fmt` itself would make.
pub struct Generic<'a, W: ?Sized>(pub &'a mut W);

/// Each method is out of line, one copy per destination type, so that a piece costs
/// its call site one call: inlined, a `String`'s `push_str` was repeated at every
/// piece, and an image that links `core::fmt` anyway grew instead of shrinking.
impl<W: fmt::Write + ?Sized> Sink for Generic<'_, W> {
    #[inline(never)]
    fn put_str(&mut self, s: &str) -> fmt::Result {
        self.0.write_str(s)
    }

    #[inline(never)]
    fn put_char(&mut self, c: char) -> fmt::Result {
        self.0.write_char(c)
    }

    #[inline(never)]
    fn put_u32(&mut self, magnitude: u32, negative: bool) -> fmt::Result {
        if negative {
            self.0.write_char('-')?;
        }
        self.0.write_str(Decimal::of_u32(magnitude).as_str())
    }

    #[inline(never)]
    fn put_i64(&mut self, value: i64) -> fmt::Result {
        if value < 0 {
            self.0.write_char('-')?;
        }
        self.0
            .write_str(Decimal::of_u64(value.unsigned_abs()).as_str())
    }

    #[inline(never)]
    fn put_u64(&mut self, value: u64) -> fmt::Result {
        self.0.write_str(Decimal::of_u64(value).as_str())
    }
}
