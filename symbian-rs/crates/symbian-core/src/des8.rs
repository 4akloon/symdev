//! The 8-bit descriptor family, as much of it as the file API needs: a borrowed view of
//! bytes to write, and a writable view of a byte buffer to read into.
//!
//! Where the 16-bit family in [`crate::des`] builds its header word itself — the type
//! nibbles were observed on the device in experiment 69 — the 8-bit nibbles were never
//! observed. So these two types never write a header word at all: they hand zeroed
//! storage of the measured size to euser's own exported constructor and let it build
//! the descriptor. The only thing read back is the length, through the mask
//! `e32des8.h` declares publicly.
//!
//! Both types are borrows with a lifetime, like `TPtrC8` and `TPtr8` themselves: the
//! bytes live in the caller's buffer and are never copied.
use core::marker::PhantomData;

use symbian_sys::des8::{
    MASK_DES_LENGTH8, TDes8, TDesC8, TPtr8_ctor, TPtr8Storage, TPtrC8_ctor, TPtrC8Storage,
};

use crate::{ErrorKind, Result, SymbianError};

/// Anything that can be handed to a Symbian API taking `const TDesC8&`.
pub trait DesC8 {
    /// The `const TDesC8&` the API expects: a pointer to the header word.
    fn as_tdesc8(&self) -> *const TDesC8;

    /// The number of bytes the descriptor currently holds.
    fn len(&self) -> usize;

    /// Whether the descriptor holds no bytes.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// `KErrTooBig` unless `len` fits both the 28-bit length field and a `TInt`.
fn length_of(len: usize) -> Result<i32> {
    if len > MASK_DES_LENGTH8 as usize || len > i32::MAX as usize {
        return Err(SymbianError::of(ErrorKind::TooBig));
    }
    Ok(len as i32)
}

/// A read-only `TPtrC8` over bytes that live somewhere else: the argument of
/// `RFile::Write`.
pub struct PtrC8<'a> {
    storage: TPtrC8Storage,
    _bytes: PhantomData<&'a [u8]>,
}

impl<'a> PtrC8<'a> {
    /// Points at `bytes` for as long as they live.
    ///
    /// `KErrTooBig` for more bytes than a descriptor length can express.
    pub fn new(bytes: &'a [u8]) -> Result<Self> {
        let length = length_of(bytes.len())?;
        let mut storage = TPtrC8Storage::zeroed();
        // SAFETY: `storage` is zeroed storage of the measured `sizeof(TPtrC8)` and
        // 4-aligned, so it is a valid `TPtrC8` object to construct into; `this` is
        // argument 0 under the observed member ABI. The constructor only writes the two
        // words of the descriptor and stores `bytes`' address, which `PhantomData` keeps
        // borrowed for `'a`. It cannot leave: `e32des8.h` declares it `IMPORT_C
        // TPtrC8(const TUint8*, TInt)` with no leave, and euser's descriptor
        // constructors do not allocate.
        unsafe { TPtrC8_ctor(&mut storage, bytes.as_ptr(), length) };
        Ok(Self {
            storage,
            _bytes: PhantomData,
        })
    }
}

impl DesC8 for PtrC8<'_> {
    fn as_tdesc8(&self) -> *const TDesC8 {
        self.storage.as_tdesc8()
    }

    fn len(&self) -> usize {
        self.storage.length()
    }
}

/// A writable `TPtr8` over a caller's buffer: the argument of `RFile::Read`, which
/// fills it up to `iMaxLength` and sets its length.
///
/// It starts empty with `iMaxLength` equal to the buffer's length, so a read never
/// writes outside the slice.
pub struct Ptr8<'a> {
    storage: TPtr8Storage,
    _bytes: PhantomData<&'a mut [u8]>,
}

impl<'a> Ptr8<'a> {
    /// An empty descriptor whose maximum length is `bytes.len()`.
    pub fn new(bytes: &'a mut [u8]) -> Result<Self> {
        let max_length = length_of(bytes.len())?;
        let mut storage = TPtr8Storage::zeroed();
        // SAFETY: as `PtrC8::new`, with the measured `sizeof(TPtr8)` and the three-
        // argument constructor `TPtr8(TUint8*, TInt aLength, TInt aMaxLength)`. The
        // length is 0 and the maximum is the slice's own length, so every byte euser
        // may write is inside the slice, which is borrowed mutably for `'a`.
        unsafe { TPtr8_ctor(&mut storage, bytes.as_mut_ptr(), 0, max_length) };
        Ok(Self {
            storage,
            _bytes: PhantomData,
        })
    }

    /// The `TDes8&` a filling export expects.
    pub fn as_tdes8(&mut self) -> *mut TDes8 {
        self.storage.as_tdes8()
    }
}

impl DesC8 for Ptr8<'_> {
    fn as_tdesc8(&self) -> *const TDesC8 {
        self.storage.as_tdesc8()
    }

    fn len(&self) -> usize {
        self.storage.length()
    }
}
