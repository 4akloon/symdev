//! `HeapBytes`: bytes read out of a file into one heap cell of their own.
use core::ptr::NonNull;

use symbian_sys::euser::{User_Alloc, User_Free};

use super::layout::{ReadAt, Span};
use crate::error::Result;
use crate::{ErrorKind, SymbianError};

/// Exactly `span.len` bytes of a file in one `User::Alloc` cell, freed on drop. No
/// descriptor header, so the cell is the bytes alone; an empty span holds no cell.
pub(super) struct HeapBytes {
    bytes: NonNull<u8>,
    len: usize,
}

impl HeapBytes {
    /// Allocates and fills the cell. `KErrNoMemory` on a full heap; a file that ends
    /// early is the reader's `KErrCorrupt`.
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
        // From here the cell is this value's, so every early return frees it.
        let mut out = Self { bytes, len };
        file.read_exact_at(span.at, out.as_mut())?;
        Ok(out)
    }

    fn as_mut(&mut self) -> &mut [u8] {
        // SAFETY: `bytes` is a `len`-byte cell this value owns (or dangling with `len`
        // 0). `User::Alloc` does not initialise it, and this slice is only written
        // through before anything reads it: `read` hands it to `read_exact_at` first.
        unsafe { core::slice::from_raw_parts_mut(self.bytes.as_ptr(), self.len) }
    }

    pub(super) fn as_bytes(&self) -> &[u8] {
        // SAFETY: `bytes` is `len` bytes this value owns, filled by `read` and alive
        // until `drop`.
        unsafe { core::slice::from_raw_parts(self.bytes.as_ptr(), self.len) }
    }
}

impl Drop for HeapBytes {
    fn drop(&mut self) {
        if self.len != 0 {
            // SAFETY: a non-empty value owns the cell `User::Alloc` returned in `read` on
            // this thread's heap, and is its only owner.
            unsafe { User_Free(self.bytes.as_ptr()) }
        }
    }
}
