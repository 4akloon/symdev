//! `&str` ↔ UTF-16 with no allocation: the caller always provides the buffer.
use core::str;

use crate::{ErrorKind, Result, SymbianError};

/// How many UTF-16 code units `s` needs. Symbian text is UTF-16, so a `&str` outside the
/// Basic Multilingual Plane costs two units per character, not one.
pub fn utf16_len(s: &str) -> usize {
    s.chars().map(char::len_utf16).sum()
}

/// Writes `s` as UTF-16 into `out` and returns how many code units it used.
///
/// Fails with `KErrOverflow` when `out` is too small; nothing is allocated and `out` is
/// left holding whatever was written before the overflow was noticed.
pub fn encode_utf16_into(s: &str, out: &mut [u16]) -> Result<usize> {
    let mut n = 0;
    for unit in s.encode_utf16() {
        let Some(slot) = out.get_mut(n) else {
            return Err(SymbianError::of(ErrorKind::Overflow));
        };
        *slot = unit;
        n += 1;
    }
    Ok(n)
}

/// Decodes UTF-16 code units into `out` as UTF-8 and returns them as a `&str`.
///
/// Fails with `KErrArgument` for an unpaired surrogate and `KErrOverflow` when `out` is
/// too small. Nothing is allocated.
pub fn decode_utf16_into<'b>(units: &[u16], out: &'b mut [u8]) -> Result<&'b str> {
    let mut n = 0;
    for decoded in char::decode_utf16(units.iter().copied()) {
        let c = decoded.map_err(|_| SymbianError::of(ErrorKind::Argument))?;
        let width = c.len_utf8();
        let Some(slot) = out.get_mut(n..n + width) else {
            return Err(SymbianError::of(ErrorKind::Overflow));
        };
        c.encode_utf8(slot);
        n += width;
    }
    // `encode_utf8` only ever writes well-formed UTF-8, so this cannot fail; mapping it
    // rather than asserting keeps the crate free of panics.
    str::from_utf8(&out[..n]).map_err(|_| SymbianError::of(ErrorKind::Corrupt))
}
