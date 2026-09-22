//! `Buf16` as the fast `write!`'s native destination (experiment 101): text through
//! [`Buf16::push_str`], numbers through euser's `TDes16::AppendNum`, which is code in
//! ROM, so an image that formats only strings and integers into a `Buf16` links none
//! of `core::fmt`.
//!
//! Every method leaves the buffer as `core::write!` would, failures included, since
//! `write!`'s `fmt::Write` impl for `Buf16` is `push_str` and `write_char` its default
//! (`push_str` of the character's UTF-8).

use core::fmt;

use symbian_fmt::__private::Decimal;
use symbian_fmt::Sink;

use super::Buf16;

impl<const N: usize> Sink for Buf16<N> {
    fn put_str(&mut self, s: &str) -> fmt::Result {
        self.push_str(s).map_err(|_| fmt::Error)
    }

    /// Text known at compile time: its UTF-16 is copied, nothing is transcoded.
    #[inline(never)]
    fn put_utf16(&mut self, _text: &str, units: &[u16]) -> fmt::Result {
        self.append_units(units).map_err(|_| fmt::Error)
    }

    fn put_char(&mut self, c: char) -> fmt::Result {
        self.push(c).map_err(|_| fmt::Error)
    }

    fn put_u32(&mut self, magnitude: u32, negative: bool) -> fmt::Result {
        let value = i64::from(magnitude);
        self.put_i64(if negative { -value } else { value })
    }

    fn put_i64(&mut self, value: i64) -> fmt::Result {
        if self.append_num(value).is_ok() {
            return Ok(());
        }
        // `core` writes the sign with its own `write_char` before the digits, so when
        // the number does not fit, a `-` that does is left behind.
        if value < 0 {
            self.push('-').map_err(|_| fmt::Error)?;
        }
        Err(fmt::Error)
    }

    fn put_u64(&mut self, value: u64) -> fmt::Result {
        self.push_str(Decimal::of_u64(value).as_str())
            .map_err(|_| fmt::Error)
    }
}
