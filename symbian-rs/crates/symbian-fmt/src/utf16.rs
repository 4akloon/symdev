//! Text known at compile time, stored as UTF-16 too (experiment 106).
//!
//! Every Symbian text API takes UTF-16 and Rust text is UTF-8, so appending a `&str` to
//! a descriptor transcodes it at run time. C++ does not: `_LIT(KText, "…")` makes the
//! compiler put UTF-16 into the image. A [`Utf16Str`] is the same thing for Rust — the
//! `&'static str` and its UTF-16 code units, both computed at compile time by
//! [`utf16!`](crate::utf16) — and a destination that holds UTF-16 (`Buf16`) copies the
//! units instead of converting the text. Every other destination is given the `&str`,
//! so what it receives is exactly what the `&str` would have given it.

use core::fmt;
use core::ops::Deref;

/// A `&'static str` together with its UTF-16 code units, both in the image.
///
/// Built only by [`utf16!`](crate::utf16), which encodes the text at compile time, so
/// the two always hold the same text. It dereferences to the `str` and displays as the
/// `str`, so `GREETING.len()` and `{GREETING}` read as they did for a `&str`.
#[derive(Clone, Copy)]
pub struct Utf16Str {
    text: &'static str,
    units: &'static [u16],
}

impl Utf16Str {
    /// What [`utf16!`](crate::utf16) expands to. Not an API: `units` must be
    /// `encode_utf16(text)`, which only the macro guarantees.
    #[doc(hidden)]
    pub const fn __encoded(text: &'static str, units: &'static [u16]) -> Self {
        Self { text, units }
    }

    /// The text as UTF-8.
    pub const fn as_str(&self) -> &'static str {
        self.text
    }

    /// The text as UTF-16 code units.
    pub const fn units(&self) -> &'static [u16] {
        self.units
    }
}

impl Deref for Utf16Str {
    type Target = str;

    fn deref(&self) -> &str {
        self.text
    }
}

impl fmt::Display for Utf16Str {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self.text, f)
    }
}

impl fmt::Debug for Utf16Str {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.text, f)
    }
}

/// A [`Utf16Str`] of a `&'static str` constant expression — a literal, a `concat!`, or
/// the name of a `const &str` — encoded at compile time:
///
/// ```ignore
/// const GREETING: Utf16Str = utf16!("Hello from Rust SDK");
/// ```
#[macro_export]
macro_rules! utf16 {
    ($text:expr) => {{
        const __TEXT: &'static str = $text;
        const __UNITS: [u16; $crate::__private::utf16_len(__TEXT)] =
            $crate::__private::encode_utf16(__TEXT);
        $crate::Utf16Str::__encoded(__TEXT, &__UNITS)
    }};
}

/// How many UTF-16 code units `s` takes: one per character, two above U+FFFF (a UTF-8
/// lead byte of `0xF0` or more).
pub const fn utf16_len(s: &str) -> usize {
    let bytes = s.as_bytes();
    let (mut i, mut n) = (0, 0);
    while i < bytes.len() {
        let b = bytes[i];
        if b & 0xc0 != 0x80 {
            n += if b >= 0xf0 { 2 } else { 1 };
        }
        i += 1;
    }
    n
}

/// `s` as UTF-16, for `N = utf16_len(s)`. Evaluated in a `const` item by
/// [`utf16!`](crate::utf16); a `&str` is well-formed UTF-8, so every continuation byte
/// it reads is there, and a unit past `N` (which the macro never asks for) is dropped
/// rather than written.
pub const fn encode_utf16<const N: usize>(s: &str) -> [u16; N] {
    let bytes = s.as_bytes();
    let mut out = [0u16; N];
    let (mut i, mut n) = (0, 0);
    while i < bytes.len() {
        let lead = bytes[i] as u32;
        let (width, bits) = match lead {
            0x00..=0x7f => (1, lead),
            0x80..=0xdf => (2, lead & 0x1f),
            0xe0..=0xef => (3, lead & 0x0f),
            _ => (4, lead & 0x07),
        };
        let mut c = bits;
        let mut k = 1;
        while k < width {
            c = (c << 6) | (continuation(bytes, i + k) & 0x3f);
            k += 1;
        }
        i += width;
        if c >= 0x1_0000 {
            let v = c - 0x1_0000;
            n = put(&mut out, n, 0xd800 | (v >> 10) as u16);
            n = put(&mut out, n, 0xdc00 | (v & 0x3ff) as u16);
        } else {
            n = put(&mut out, n, c as u16);
        }
    }
    out
}

const fn continuation(bytes: &[u8], at: usize) -> u32 {
    if at < bytes.len() { bytes[at] as u32 } else { 0 }
}

const fn put<const N: usize>(out: &mut [u16; N], at: usize, unit: u16) -> usize {
    if at < N {
        out[at] = unit;
    }
    at + 1
}
