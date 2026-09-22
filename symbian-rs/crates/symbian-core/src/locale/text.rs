//! `Text`: one string read out of the strings file, in a heap cell of its own.
use core::fmt;
use core::ops::Deref;

use super::heap_bytes::HeapBytes;
use crate::error::Result;
use crate::{ErrorKind, SymbianError};

/// A string read from the strings file: its UTF-8 in one heap cell, freed on drop. An
/// empty string holds no cell.
///
/// It dereferences to `str`; the bytes were checked to be UTF-8 once, when read.
pub struct Text {
    bytes: HeapBytes,
}

impl Text {
    /// The resource's bytes, if they are UTF-8; `KErrCorrupt` otherwise.
    pub(super) fn new(bytes: HeapBytes) -> Result<Self> {
        match core::str::from_utf8(bytes.as_bytes()) {
            Ok(_) => Ok(Self { bytes }),
            Err(_) => Err(SymbianError::of(ErrorKind::Corrupt)),
        }
    }
}

impl Deref for Text {
    type Target = str;

    fn deref(&self) -> &str {
        // SAFETY: `new` refused any bytes that are not UTF-8, and they do not change
        // afterwards.
        unsafe { core::str::from_utf8_unchecked(self.bytes.as_bytes()) }
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
