//! `TPckgBuf<T>`: a `T` with a descriptor header in front of it, which is how the socket
//! server returns a structure — `TNameEntry` (a `TPckgBuf<TNameRecord>`) and
//! `TSockXfrLength` (a `TPckgBuf<TInt>`).
//!
//! **The header word is not written by Rust.** `TPckgBuf<T>::TPckgBuf()` is inline and
//! `e32cmn.inl` line 2659 spells out exactly what it does:
//!
//! ```cpp
//! inline TPckgBuf<T>::TPckgBuf() : TAlignedBuf8<sizeof(T)>(sizeof(T)) {}
//! inline TAlignedBuf8<S>::TAlignedBuf8(TInt aLength) : TBufBase8(aLength, S) {}
//! ```
//!
//! and `TBufBase8(TInt aLength, TInt aMaxLength)` is `IMPORT_C` (`e32des8.h` line 457),
//! exported by euser as `_ZN9TBufBase8C1Eii`. So a packaged buffer is built the same way
//! the 8-bit descriptors in [`crate::des8`] are: zeroed storage of the measured size,
//! then euser's own constructor, with the type nibble never guessed.
//!
//! The storage must be **8-aligned**: `TAlignedBuf8` exists only to force that
//! (`e32des8.h` lines 502-530, the union with a `double`), so that a `T` with 64-bit
//! members inside the buffer is aligned.

use crate::des8::{TDes8, TDesC8};

/// Storage for a `TSockXfrLength` — `TPckgBuf<TInt>`, `sizeof == 16`, `alignof == 8`,
/// measured on the recorded GCCE argv.
///
/// `RSocket::RecvOneOrMore` reports the number of bytes it moved here. The payload is
/// the `TInt` at offset 8, after the header's `iLength`/`iMaxLength` pair.
#[repr(C, align(8))]
pub struct TSockXfrLengthStorage {
    bytes: [u8; 16],
}

/// `sizeof(TInt)`: the `aLength`/`aMaxLength` a `TPckgBuf<TInt>` is constructed with.
const XFR_LENGTH_PAYLOAD: i32 = 4;

/// Where a `TPckgBuf<T>`'s `T` begins: **8**, measured rather than derived, by compiling
/// `(TUint8*)&(((TSockXfrLength*)0)->operator()()) - (TUint8*)0` with the recorded GCCE
/// argv — and the same number for `TNameEntry`. It is `sizeof(TDes8)`, also measured as
/// 8, because `TAlignedBuf8`'s union starts right after the header and both the union
/// and the whole object are 8-aligned.
pub const TPCKG_BUF_OFFSET_DATA: usize = 8;

impl TSockXfrLengthStorage {
    /// Zeroed storage, ready for [`TPckgBuf_ctor`].
    pub const fn zeroed() -> Self {
        Self { bytes: [0; 16] }
    }

    /// The `aLength`/`aMaxLength` pair [`TPckgBuf_ctor`] takes for this buffer.
    pub const fn payload_len() -> i32 {
        XFR_LENGTH_PAYLOAD
    }

    /// The `TPckgBuf<TInt>&` `RSocket::RecvOneOrMore` expects. There is no distinct
    /// `TPckgBuf` type in this crate: the export takes it by reference and does nothing
    /// with it that is not a `TDes8` operation, so the address of the storage is the
    /// argument.
    pub const fn as_tdes8(&mut self) -> *mut TDes8 {
        (self as *mut Self).cast()
    }

    /// The `TInt` the server wrote into the packaged buffer.
    pub const fn value(&self) -> i32 {
        let b = &self.bytes;
        i32::from_le_bytes([
            b[TPCKG_BUF_OFFSET_DATA],
            b[TPCKG_BUF_OFFSET_DATA + 1],
            b[TPCKG_BUF_OFFSET_DATA + 2],
            b[TPCKG_BUF_OFFSET_DATA + 3],
        ])
    }

    /// The `const TDesC8&` a reading export would expect, for symmetry with
    /// [`crate::des8`]. Unused today; kept out of the public surface until it is.
    #[allow(dead_code)]
    pub(crate) const fn as_tdesc8(&self) -> *const TDesC8 {
        (self as *const Self).cast()
    }
}

unsafe extern "C" {
    /// `000016f0 T _ZN9TBufBase8C1Eii` — `TBufBase8::TBufBase8(TInt aLength, TInt
    /// aMaxLength)`, euser.dso.
    ///
    /// The one call that turns zeroed storage into a valid `TPckgBuf<T>`: pass
    /// `sizeof(T)` for both arguments, which is what `TPckgBuf<T>()` does. `this` is
    /// argument 0 under the observed member ABI; the constructor is `protected` in C++,
    /// which is a compile-time rule and has no bearing on the exported symbol.
    #[link_name = "_ZN9TBufBase8C1Eii"]
    pub fn TPckgBuf_ctor(this: *mut u8, length: i32, max_length: i32);
}
