//! The 8-bit descriptor family (`e32des8.h`), which is how every byte reaches and
//! leaves `RFile`: `RFile::Write` takes `const TDesC8&` and `RFile::Read` fills a
//! `TDes8&`.
//!
//! **Nothing about the header word is assumed here.** Experiment 69 observed the type
//! nibbles of the *16-bit* family on the device's own euser; the 8-bit ones were never
//! observed, and guessing that the two families share an enum is exactly the kind of
//! recall this repository does not accept. So a `TPtrC8` or `TPtr8` is never built by
//! writing a header word: euser's own exported constructors build it, in place, in
//! storage Rust owns. The sizes that storage must have were measured by compiling
//! `return sizeof(TPtrC8);` with the recorded GCCE argv and reading the immediate —
//! `sizeof(TDesC8) == 4`, `sizeof(TPtrC8) == 8`,
//! `sizeof(TDes8) == 8`, `sizeof(TPtr8) == 12` (experiment 79).
//!
//! The only field this module reads directly is the length, and only through the two
//! constants the public header declares: `KMaskDesLength8 = 0xfffffff` and
//! `KShiftDesType8 = 28` (`e32des8.h` lines 12 and 16).
//!
//! Every constructor below is a non-static member function called with `this` as
//! argument 0 — the member ABI observed in experiment 78, not guessed.

/// `KMaskDesLength8` (`e32des8.h` line 12): the low bits of the header word that hold
/// the length, and therefore the greatest length an 8-bit descriptor can have.
pub const MASK_DES_LENGTH8: u32 = 0x0fff_ffff;

/// `KShiftDesType8` (`e32des8.h` line 16).
pub const SHIFT_DES_TYPE8: u32 = 28;

/// The opaque `TDesC8` an export takes by `const&`; only ever seen behind a pointer.
#[repr(C)]
pub struct TDesC8 {
    _private: [u8; 0],
}

/// The opaque `HBufC8` (`e32des8.h` line 345): a heap descriptor whose header and bytes
/// are one cell on the thread's heap. Only ever seen behind a pointer, as the result of
/// an allocating export such as `RResourceFile::AllocReadL`; an `HBufC8` is a `TDesC8`,
/// so the pointer is also a valid `const TDesC8*`.
#[repr(C)]
pub struct HBufC8 {
    _private: [u8; 0],
}

/// The opaque `TDes8` a filling export takes by `&`; only ever seen behind a pointer.
#[repr(C)]
pub struct TDes8 {
    _private: [u8; 0],
}

/// Storage for a `TPtrC8`: `sizeof(TPtrC8) == 8`, measured.
///
/// Word 0 is the header (`iTypeLength`) and word 1 the data pointer, but this type never
/// writes either: [`TPtrC8_ctor`] does, and only [`TPtrC8Storage::length`] reads, through
/// the header's documented mask.
#[repr(C, align(4))]
pub struct TPtrC8Storage {
    words: [u32; 2],
}

/// Storage for a `TPtr8`: `sizeof(TPtr8) == 12`, measured.
#[repr(C, align(4))]
pub struct TPtr8Storage {
    words: [u32; 3],
}

impl TPtrC8Storage {
    /// Zeroed storage, ready for euser's constructor to build a descriptor into.
    pub const fn zeroed() -> Self {
        Self { words: [0; 2] }
    }

    /// The `const TDesC8&` an export expects.
    pub const fn as_tdesc8(&self) -> *const TDesC8 {
        (self as *const Self).cast()
    }

    /// The length in bytes, from the header word's documented low 28 bits.
    pub const fn length(&self) -> usize {
        (self.words[0] & MASK_DES_LENGTH8) as usize
    }
}

impl TPtr8Storage {
    /// Zeroed storage, ready for euser's constructor to build a descriptor into.
    pub const fn zeroed() -> Self {
        Self { words: [0; 3] }
    }

    /// The `const TDesC8&` an export expects: a `TPtr8` is a `TDes8` is a `TDesC8`, and
    /// all three begin at the header word (`sizeof(TDesC8) == 4`, measured).
    pub const fn as_tdesc8(&self) -> *const TDesC8 {
        (self as *const Self).cast()
    }

    /// The `TDes8&` a filling export expects.
    pub const fn as_tdes8(&mut self) -> *mut TDes8 {
        (self as *mut Self).cast()
    }

    /// The length in bytes, from the header word's documented low 28 bits.
    pub const fn length(&self) -> usize {
        (self.words[0] & MASK_DES_LENGTH8) as usize
    }
}

unsafe extern "C" {
    /// `00001020 T _ZN6TPtrC8C1EPKhi` — `TPtrC8::TPtrC8(const TUint8* aBuf, TInt
    /// aLength)`, euser.dso. Builds the descriptor in `this`, so Rust never has to know
    /// the type nibble.
    #[link_name = "_ZN6TPtrC8C1EPKhi"]
    pub fn TPtrC8_ctor(this: *mut TPtrC8Storage, buf: *const u8, length: i32);

    /// `00000cbc T _ZN5TPtr8C1EPhii` — `TPtr8::TPtr8(TUint8* aBuf, TInt aLength, TInt
    /// aMaxLength)`, euser.dso.
    #[link_name = "_ZN5TPtr8C1EPhii"]
    pub fn TPtr8_ctor(this: *mut TPtr8Storage, buf: *mut u8, length: i32, max_length: i32);

    /// `00000c34 T _ZN5TDes89SetLengthEi` — `TDes8::SetLength(TInt aLength)`, euser.dso.
    ///
    /// Panics (`ETDes8Overflow`, `e32panic.h`) for a length above `iMaxLength`, which no
    /// `TRAP` catches, so a caller must check the room first.
    #[link_name = "_ZN5TDes89SetLengthEi"]
    pub fn TDes8_SetLength(this: *mut TDes8, length: i32);
}
