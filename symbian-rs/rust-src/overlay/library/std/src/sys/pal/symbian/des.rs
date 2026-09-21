//! The two descriptors the platform layer needs: a UTF-16 path buffer to hand to the
//! file server, and an 8-bit view over a byte slice to hand to `RFile`.
//!
//! This is the small, `std`-facing part of what `symbian_core::des` does for the
//! `no_std` SDK. The layouts are the ones **observed** on this ROM's own euser in
//! experiment 69, not read out of a header that does not state them: a descriptor
//! begins with one `TUint` whose top nibble is the type and whose low 28 bits are the
//! length, with `EBuf = 3` for a `TBuf16<N>` and the code units at offset 8, after
//! `iMaxLength`.
//!
//! The 8-bit family is different and deliberately so: its type nibbles were never
//! observed, so nothing here writes an 8-bit header word. euser's own exported
//! constructors build a `TPtrC8`/`TPtr8` in place, in storage this module owns, in
//! sizes that were measured by compiling `sizeof` with the recorded GCCE argv
//! (experiment 79).

use crate::io;
use symbian_sys::des::TDesC16;
use symbian_sys::des8::{
    TDes8, TDesC8, TPtr8Storage, TPtrC8Storage, TPtr8_ctor, TPtrC8_ctor,
};

/// `KShiftDesType16` (`e32des16.h`).
const TYPE_SHIFT: u32 = 28;
/// `EBuf`, the type nibble of a `TBuf16<N>` (observed, experiment 69).
const EBUF: u32 = 3;

/// `KMaxFileName` (`f32file.h`): the longest path the file server accepts.
pub const MAX_FILE_NAME: usize = 256;

/// A `TBuf16<256>` on the stack: a Symbian path, built from a Rust `&str`.
///
/// `std` hands the platform layer a `&Path`, whose bytes are UTF-8; the file server
/// wants UTF-16 in a descriptor. This does the transcoding with no allocation, which is
/// what lets `File::open` work when the heap is exhausted.
#[repr(C)]
pub struct PathBuf16 {
    type_length: u32,
    max_length: i32,
    units: [u16; MAX_FILE_NAME],
}

impl PathBuf16 {
    /// The path as a descriptor, or `InvalidFilename` if it does not fit in
    /// `KMaxFileName` code units.
    ///
    /// A path longer than that is not a path the file server could ever have opened, so
    /// reporting it here rather than truncating is the only honest answer — and a
    /// truncated path would name a *different* file.
    pub fn new(path: &str) -> io::Result<Self> {
        let mut buf =
            PathBuf16 { type_length: 0, max_length: MAX_FILE_NAME as i32, units: [0; MAX_FILE_NAME] };
        let mut len = 0usize;
        for unit in path.encode_utf16() {
            if len >= MAX_FILE_NAME {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidFilename,
                    "a Symbian path is at most KMaxFileName = 256 UTF-16 code units",
                ));
            }
            buf.units[len] = unit;
            len += 1;
        }
        buf.type_length = (EBUF << TYPE_SHIFT) | len as u32;
        Ok(buf)
    }

    /// The `const TDesC16&` an efsrv export expects: a pointer to the header word.
    pub fn as_tdesc16(&self) -> *const TDesC16 {
        (self as *const Self).cast()
    }
}

/// A `TPtrC8` over a byte slice: what `RFile::Write` takes.
pub struct Bytes<'a> {
    storage: TPtrC8Storage,
    _borrow: core::marker::PhantomData<&'a [u8]>,
}

impl<'a> Bytes<'a> {
    pub fn new(bytes: &'a [u8]) -> io::Result<Self> {
        let len = i32::try_from(bytes.len())
            .map_err(|_| io::Error::new(io::ErrorKind::FileTooLarge, "more bytes than a TInt"))?;
        let mut storage = TPtrC8Storage::zeroed();
        // SAFETY: `TPtrC8::TPtrC8(const TUint8*, TInt)` is a non-static member built in
        // place at `this` (the member ABI observed in experiment 78). The storage is
        // `sizeof(TPtrC8)` bytes, measured, and the borrow keeps `bytes` alive for as
        // long as the descriptor can be used.
        unsafe { TPtrC8_ctor(&mut storage, bytes.as_ptr(), len) };
        Ok(Self { storage, _borrow: core::marker::PhantomData })
    }

    pub fn as_tdesc8(&self) -> *const TDesC8 {
        self.storage.as_tdesc8()
    }
}

/// A `TPtr8` over a mutable byte slice: what `RFile::Read` fills.
pub struct BytesMut<'a> {
    storage: TPtr8Storage,
    _borrow: core::marker::PhantomData<&'a mut [u8]>,
}

impl<'a> BytesMut<'a> {
    pub fn new(bytes: &'a mut [u8]) -> io::Result<Self> {
        let max = i32::try_from(bytes.len())
            .map_err(|_| io::Error::new(io::ErrorKind::FileTooLarge, "more bytes than a TInt"))?;
        let mut storage = TPtr8Storage::zeroed();
        // SAFETY: as `Bytes::new`; `TPtr8::TPtr8(TUint8*, TInt aLength, TInt aMaxLength)`
        // is built in place in storage of the measured `sizeof(TPtr8)`, with a length of
        // zero so that `RFile::Read` fills it from the start.
        unsafe { TPtr8_ctor(&mut storage, bytes.as_mut_ptr(), 0, max) };
        Ok(Self { storage, _borrow: core::marker::PhantomData })
    }

    pub fn as_tdes8(&mut self) -> *mut TDes8 {
        self.storage.as_tdes8()
    }

    /// How many bytes the descriptor says it holds, read through the documented length
    /// mask and nothing else.
    pub fn len(&self) -> usize {
        self.storage.length()
    }
}
