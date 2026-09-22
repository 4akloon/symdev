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

    /// An integer's `Display` with no flags: `core` writes the sign with
    /// `write_char('-')` and then the digits with one `write_str`, so a destination
    /// with room for the sign and not the digits ends up holding the `-`.
    fn put_int(&mut self, magnitude: u64, negative: bool) -> fmt::Result;
}

/// Any `fmt::Write`, driven through the calls `core::fmt` itself would make.
pub struct Generic<'a, W: ?Sized>(pub &'a mut W);

impl<W: fmt::Write + ?Sized> Sink for Generic<'_, W> {
    fn put_str(&mut self, s: &str) -> fmt::Result {
        self.0.write_str(s)
    }

    fn put_char(&mut self, c: char) -> fmt::Result {
        self.0.write_char(c)
    }

    fn put_int(&mut self, magnitude: u64, negative: bool) -> fmt::Result {
        if negative {
            self.0.write_char('-')?;
        }
        self.0.write_str(Decimal::of(magnitude).as_str())
    }
}
