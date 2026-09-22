//! `Text`: one string read out of the strings file, in a heap cell of its own.
use core::fmt;
use core::ops::Deref;
use core::ptr::NonNull;

use symbian_sys::euser::{User_Alloc, User_Free};

use super::layout::{ReadAt, Span};
use crate::error::Result;
use crate::{ErrorKind, SymbianError};

/// A string read from the strings file: its UTF-8 in one heap cell, freed on drop. An
/// empty string holds no cell.
///
/// It dereferences to `str`; the bytes were checked to be UTF-8 once, when read.
pub struct Text {
    bytes: NonNull<u8>,
    len: usize,
}

impl Text {
    /// Allocates exactly `span.len` bytes and reads the resource into them. The cell is
    /// `User::Alloc`'s, as `RResourceFile::AllocReadL`'s was, but holds the bytes alone —
    /// no `HBufC8` header.
    pub(super) fn read<R: ReadAt<Error = SymbianError>>(file: &R, span: Span) -> Result<Self> {
        let len = span.len as usize;
        let bytes = if len == 0 {
            NonNull::dangling()
        } else {
            let size = i32::try_from(len).map_err(|_| SymbianError::of(ErrorKind::TooBig))?;
            // SAFETY: `User::Alloc` takes a size and returns a cell of at least that many
            // bytes on this thread's heap, or null; it does not leave.
            let cell = unsafe { User_Alloc(size) };
            NonNull::new(cell).ok_or(SymbianError::of(ErrorKind::NoMemory))?
        };
        // From here the cell is `Text`'s, so every early return frees it.
        let text = Self { bytes, len };
        // SAFETY: the cell is `len` bytes this value owns and nothing else refers to;
        // `User::Alloc` does not initialise it, so it is only written through this
        // slice before it is ever read.
        let buf = unsafe { core::slice::from_raw_parts_mut(text.bytes.as_ptr(), len) };
        file.read_exact_at(span.at, buf)?;
        match core::str::from_utf8(text.as_bytes()) {
            Ok(_) => Ok(text),
            Err(_) => Err(SymbianError::of(ErrorKind::Corrupt)),
        }
    }

    fn as_bytes(&self) -> &[u8] {
        // SAFETY: `bytes` is `len` initialised bytes this value owns (or dangling with
        // `len` 0), alive until `drop` and never written after `read`.
        unsafe { core::slice::from_raw_parts(self.bytes.as_ptr(), self.len) }
    }
}

impl Deref for Text {
    type Target = str;

    fn deref(&self) -> &str {
        // SAFETY: `read` refused any bytes that are not UTF-8, and they do not change
        // afterwards.
        unsafe { core::str::from_utf8_unchecked(self.as_bytes()) }
    }
}

impl fmt::Display for Text {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&**self, f)
    }
}

impl fmt::Debug for Text {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

impl Drop for Text {
    fn drop(&mut self) {
        if self.len != 0 {
            // SAFETY: a non-empty `Text` owns the cell `User::Alloc` returned in `read`
            // on this thread's heap, and this is its only owner.
            unsafe { User_Free(self.bytes.as_ptr()) }
        }
    }
}
