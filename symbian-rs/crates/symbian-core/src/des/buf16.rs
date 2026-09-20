//! `Buf16<N>`: a `TBuf16<N>` on the stack, the descriptor most Symbian code uses.
use core::fmt;

use symbian_sys::des::TDesC16;
use symbian_sys::des16::{TDes16, TDes16_Append, TDes16_AppendNum, TDes16_Copy, TDes16_Num};

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

/// The euser descriptor operations: text and numbers built by the ROM's own code, so a
/// program that uses them instead of `write!` links none of `core::fmt`.
///
/// Every one of them is a **non-static** `IMPORT_C` member function of `TDes16`, called
/// directly with `this` as argument 0 — the member ABI observed in experiment 78, not
/// guessed; the evidence is in [`symbian_sys::des16`]. None of them can leave, so none
/// of them goes through the C++ shim.
///
/// They can, however, **panic**: overflowing the destination is `ETDes16Overflow = 11`
/// in the USER category (`e32panic.h`), a panic and not a leave, which no `TRAP` can
/// catch and which takes the thread down. Each wrapper therefore checks the room first,
/// from the length word and `N` it already has, and returns `KErrOverflow` instead. A
/// failed call has written nothing.
impl<const N: usize> Buf16<N> {
    /// The `TDes16&` a modifying member function is called on.
    fn as_tdes16(&mut self) -> *mut TDes16 {
        (self as *mut Self).cast()
    }

    /// Replaces the contents with another descriptor's (`TDes16::Copy`).
    ///
    /// Lower level than the rest of this type: an application builds text with
    /// [`Buf16::push_str`] or `write!` and never names a descriptor. This is for text
    /// that arrives *from* Symbian — an out-parameter a file or system call filled in.
    pub fn copy_des(&mut self, src: &impl DesC16) -> Result<()> {
        self.room_for(src.len(), 0)?;
        // SAFETY: `self` has the observed `TBuf16<N>` layout and `src` one of the
        // observed `TDesC16` layouts, both borrowed for the whole call. The only way
        // `Copy` can fail is an overflow, which `room_for` has just ruled out, so the
        // panic that would end the thread is unreachable. It cannot leave.
        unsafe { TDes16_Copy(self.as_tdes16(), src.as_tdesc16()) };
        Ok(())
    }

    /// Appends another descriptor's contents (`TDes16::Append`). Lower level, as
    /// [`Buf16::copy_des`]: for text that arrives from Symbian.
    pub fn append_des(&mut self, src: &impl DesC16) -> Result<()> {
        self.room_for(src.len(), self.length())?;
        // SAFETY: as `copy_from`, with the existing length counted in.
        unsafe { TDes16_Append(self.as_tdes16(), src.as_tdesc16()) };
        Ok(())
    }

    /// Appends `value` in signed decimal (`TDes16::AppendNum(TInt64)`).
    ///
    /// This is the call that keeps `core::fmt` out of a binary. `e32des16.h` has no
    /// `AppendNum(TInt)`, so an `i32` widens to the 64-bit overload — which also avoids
    /// `compiler_builtins`' 64-bit division.
    pub fn append_num(&mut self, value: i64) -> Result<()> {
        self.room_for(decimal_len(value), self.length())?;
        // SAFETY: `decimal_len` is an upper bound on what euser can write for a signed
        // decimal integer — every digit plus a sign — so the overflow panic cannot
        // happen. The 64-bit argument lands in r2:r3 under the observed member ABI.
        unsafe { TDes16_AppendNum(self.as_tdes16(), value) };
        Ok(())
    }

    /// Replaces the contents with `value` in signed decimal (`TDes16::Num(TInt64)`).
    pub fn num(&mut self, value: i64) -> Result<()> {
        self.room_for(decimal_len(value), 0)?;
        // SAFETY: as `append_num`, starting from an empty buffer.
        unsafe { TDes16_Num(self.as_tdes16(), value) };
        Ok(())
    }

    /// `KErrOverflow` unless `extra` more code units fit after `existing`.
    fn room_for(&self, extra: usize, existing: usize) -> Result<()> {
        match existing.checked_add(extra) {
            Some(end) if end <= N => Ok(()),
            _ => Err(SymbianError::of(ErrorKind::Overflow)),
        }
    }
}

/// How many code units a signed decimal `value` takes: the digits plus a minus sign.
///
/// Used as the bound for the overflow check, so it must never be too small.
/// `i64::MIN.unsigned_abs()` is why the magnitude is taken as a `u64`.
const fn decimal_len(value: i64) -> usize {
    let mut len = if value < 0 { 2 } else { 1 };
    let mut rest = value.unsigned_abs() / 10;
    while rest > 0 {
        len += 1;
        rest /= 10;
    }
    len
}
