//! `Buf16<N>`: a `TBuf16<N>` on the stack, the descriptor most Symbian code uses.
use core::fmt;

use symbian_sys::des::TDesC16;

use super::{DesC16, EBUF, MAX_LENGTH, header, sealed, utf16};
use crate::{ErrorKind, Result, SymbianError};

/// A modifiable descriptor holding up to `N` UTF-16 code units inside itself.
///
/// The layout is the observed `TBuf16<N>`: the header word (`0x3xxxxxxx`, `EBuf`), then
/// `iMaxLength`, then the code units at offset 8. C++ rounds an odd `N` up to an even
/// number of units (`__Align16`), which changes only `sizeof`; here the array is exactly
/// `N` units and `iMaxLength` is `N`, so a Symbian API that respects `iMaxLength` — as
/// every descriptor function does — stays inside the array.
#[repr(C)]
pub struct Buf16<const N: usize> {
    type_length: u32,
    max_length: i32,
    buf: [u16; N],
}

impl<const N: usize> Buf16<N> {
    /// `N` has to fit both the 28-bit length field and `iMaxLength`'s `TInt`. The check
    /// is an associated constant, so it is a compile error for the offending `N` rather
    /// than a panic at run time.
    const LENGTH_FITS: () = assert!(
        N <= MAX_LENGTH && N <= i32::MAX as usize,
        "Buf16<N>: N is larger than a descriptor length"
    );

    /// An empty buffer.
    pub const fn new() -> Self {
        let () = Self::LENGTH_FITS;
        Self {
            type_length: header(EBUF, 0),
            max_length: N as i32,
            buf: [0; N],
        }
    }

    /// The greatest number of code units this buffer can hold (`iMaxLength`).
    pub const fn capacity(&self) -> usize {
        N
    }

    /// Drops everything the buffer holds.
    pub const fn clear(&mut self) {
        self.type_length = header(EBUF, 0);
    }

    /// The header word, for a layout check.
    pub const fn header_word(&self) -> u32 {
        self.type_length
    }

    const fn length(&self) -> usize {
        (self.type_length & MAX_LENGTH as u32) as usize
    }

    /// Appends `s` as UTF-16.
    ///
    /// Fails with `KErrOverflow` if it does not fit, leaving the buffer as it was: the
    /// text is measured before anything is written, so a failed append is not a partial
    /// one. Nothing is allocated.
    pub fn push_str(&mut self, s: &str) -> Result<()> {
        let len = self.length();
        let needed = utf16::utf16_len(s);
        let Some(end) = len.checked_add(needed).filter(|end| *end <= N) else {
            return Err(SymbianError::of(ErrorKind::Overflow));
        };
        utf16::encode_utf16_into(s, &mut self.buf[len..end])?;
        self.type_length = header(EBUF, end);
        Ok(())
    }

    /// Appends one character as UTF-16.
    pub fn push(&mut self, c: char) -> Result<()> {
        let mut utf8 = [0u8; 4];
        self.push_str(c.encode_utf8(&mut utf8))
    }

    /// Fills `out` with the buffer's text as UTF-8 and returns it. Nothing is allocated.
    pub fn to_str<'b>(&self, out: &'b mut [u8]) -> Result<&'b str> {
        utf16::decode_utf16_into(self.units(), out)
    }
}

impl<const N: usize> Default for Buf16<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> sealed::Sealed for Buf16<N> {}

impl<const N: usize> DesC16 for Buf16<N> {
    fn as_tdesc16(&self) -> *const TDesC16 {
        (self as *const Self).cast()
    }

    fn units(&self) -> &[u16] {
        &self.buf[..self.length()]
    }
}

/// `write!` into a `Buf16`. A buffer that runs out of room reports `fmt::Error`, which is
/// what every `core::fmt::Write` implementation does; the `KErrOverflow` behind it is
/// what [`Buf16::push_str`] returns to a caller that wants the code.
impl<const N: usize> fmt::Write for Buf16<N> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.push_str(s).map_err(|_| fmt::Error)
    }
}
