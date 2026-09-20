//! `PtrC16`: a borrowed `TPtrC16` over code units that live somewhere else.
use symbian_sys::des::TDesC16;

use super::{DesC16, EPTRC, MAX_LENGTH, header, sealed};
use crate::{ErrorKind, Result, SymbianError};

/// A non-owning descriptor over an existing `[u16]`, the shape of `TPtrC16`.
///
/// The first two fields are the whole of `TPtrC16` as observed (header word `0x1xxxxxxx`,
/// then the data pointer, `sizeof` 8). The borrowed slice is kept after them so the type
/// can hand its units back to Rust without reconstructing a slice from a raw pointer;
/// euser never looks past the two words it knows.
#[repr(C)]
pub struct PtrC16<'a> {
    type_length: u32,
    ptr: *const u16,
    units: &'a [u16],
}

impl<'a> PtrC16<'a> {
    /// Points at `units` for as long as they live.
    ///
    /// Fails with `KErrTooBig` for more units than the 28-bit length field can hold.
    pub fn new(units: &'a [u16]) -> Result<Self> {
        if units.len() > MAX_LENGTH {
            return Err(SymbianError::of(ErrorKind::TooBig));
        }
        Ok(Self {
            type_length: header(EPTRC, units.len()),
            ptr: units.as_ptr(),
            units,
        })
    }

    /// The header word, for a layout check.
    pub const fn header_word(&self) -> u32 {
        self.type_length
    }
}

impl sealed::Sealed for PtrC16<'_> {}

impl DesC16 for PtrC16<'_> {
    fn as_tdesc16(&self) -> *const TDesC16 {
        (self as *const Self).cast()
    }

    fn units(&self) -> &[u16] {
        self.units
    }
}
