//! The 16-bit descriptor family, with the layout observed on the device's own euser.
//!
//! `e32des16.h` says a descriptor begins with one `TUint` whose top nibble is the type
//! (`KShiftDesType16 = 28`) and whose low 28 bits are the length (`KMaskDesLength16`).
//! The type enum itself is in no shipped header, so experiment 69 read the header words
//! of live objects inside EKA2L1 (a C++ probe that `InfoPrint`ed them):
//!
//! | object | header word | type |
//! |---|---|---|
//! | `_LIT16("abc")` | `0x00000003` | `EBufC` = 0 |
//! | `TBufC16<8>("ab")` | `0x00000002` | `EBufC` = 0 |
//! | `TPtrC16("abcd")` | `0x10000004` | `EPtrC` = 1 |
//! | `TPtr16(p, 3, 8)` | `0x20000003` | `EPtr` = 2 |
//! | `TBuf16<8>("abc")` | `0x30000003` | `EBuf` = 3 |
//! | `HBufC16::New(8)` | `0x00000000` | `EBufC` = 0 |
//!
//! and the sizes and data offsets from the same run: `TDesC16` 4, `TPtrC16` 8 (word 1 is
//! the pointer), `TDes16` 8, `TPtr16` 12 (word 1 `iMaxLength`, word 2 the pointer),
//! `TBufC16<8>` 20 (data at +4), `TBuf16<8>` 24 (word 1 `iMaxLength`, data at +8),
//! `HBufC16` 8 (data at +4).
//!
//! `EBufCPtr` (the `RBuf16` variant) was not observed and has no type here.
mod buf16;
mod hbuf16;
mod ptrc16;
mod utf16;

pub use buf16::Buf16;
pub use hbuf16::HBuf16;
pub use ptrc16::PtrC16;
pub use utf16::{decode_utf16_into, encode_utf16_into, utf16_len};

use symbian_sys::des::{Lit16, TDesC16};

/// `KShiftDesType16` (`e32des16.h`).
pub(crate) const TYPE_SHIFT: u32 = 28;
/// `KMaskDesLength16` (`e32des16.h`): the greatest length a descriptor can hold.
pub(crate) const MAX_LENGTH: usize = 0x0fff_ffff;
/// Observed descriptor types (experiment 69, table above).
pub(crate) const EBUFC: u32 = 0;
pub(crate) const EPTRC: u32 = 1;
pub(crate) const EBUF: u32 = 3;

/// The header word of a descriptor of `kind` holding `len` code units.
pub(crate) const fn header(kind: u32, len: usize) -> u32 {
    (kind << TYPE_SHIFT) | (len as u32)
}

mod sealed {
    pub trait Sealed {}
}

/// Anything that can be handed to a Symbian API taking `const TDesC16&`.
///
/// The trait is sealed: an implementor must have one of the binary layouts observed in
/// the table above, which only this crate and `symbian-sys` provide.
pub trait DesC16: sealed::Sealed {
    /// The `const TDesC16&` euser expects: a pointer to the header word.
    fn as_tdesc16(&self) -> *const TDesC16;

    /// The code units the descriptor currently holds.
    fn units(&self) -> &[u16];

    /// The length in code units.
    fn len(&self) -> usize {
        self.units().len()
    }

    /// Whether the descriptor holds no code units.
    fn is_empty(&self) -> bool {
        self.units().is_empty()
    }
}

/// A borrowed read-only view of any descriptor: what a function takes when it does not
/// care which of the concrete shapes it was given.
///
/// It is the Rust half of `const TDesC16&` — a borrow with a lifetime — and never owns
/// or copies the text.
#[derive(Clone, Copy)]
pub struct Des16<'a>(&'a dyn DesC16);

impl<'a> Des16<'a> {
    /// Borrows a concrete descriptor for as long as it lives.
    pub fn of<D: DesC16>(des: &'a D) -> Self {
        Self(des)
    }
}

impl sealed::Sealed for Des16<'_> {}

impl DesC16 for Des16<'_> {
    fn as_tdesc16(&self) -> *const TDesC16 {
        self.0.as_tdesc16()
    }

    fn units(&self) -> &[u16] {
        self.0.units()
    }
}

impl<const N: usize> sealed::Sealed for Lit16<N> {}

impl<const N: usize> DesC16 for Lit16<N> {
    fn as_tdesc16(&self) -> *const TDesC16 {
        self.as_desc()
    }

    fn units(&self) -> &[u16] {
        Lit16::units(self).as_slice()
    }
}
