//! `TDes16`: the modifiable 16-bit descriptor's `IMPORT_C` **non-static** member
//! functions, from `nm -D epoc32/release/armv5/lib/euser.dso`.
//!
//! These are the first member functions the SDK calls directly, and they are safe to
//! call that way because the member ABI was observed rather than assumed (experiment
//! 78). A probe compiled with the recorded GCCE argv and disassembled:
//!
//! ```text
//! probe_append(TDes16* d, const TDesC16* s) { d->Append(*s); }
//!   0: push {r4, lr}
//!   2: bl _ZN6TDes166AppendERK7TDesC16      <- no register shuffle at all
//!
//! probe_appendnum(TDes16* d, TInt n) { d->AppendNum((TInt64)n); }
//!   a: movs r2, r1 ; c: asrs r3, r1, #31     <- the 64-bit argument in r2:r3, r1 skipped
//!   e: bl _ZN6TDes169AppendNumEx
//! ```
//!
//! So `this` is argument 0 under the ordinary AAPCS assignment and everything else
//! follows it: an `extern "C" fn(this, …)` is the correct declaration for a non-virtual
//! member with scalar or pointer arguments and a scalar or void return. Anything else —
//! a class returned by value (`TDesC16::Left` is sret, with `this` displaced to argument
//! 1), a `TRefByValue` varargs function such as `Format`, a virtual — needs the C++
//! shim; the rule is in `shims/common/symrs_shim.h`.
//!
//! **None of these can leave, and none of them can report failure.** Overflowing the
//! destination is `ETDes16Overflow = 11` in the USER category (`e32panic.h` line 131,
//! "any of the copying, appending or formatting member functions"), and a panic is not a
//! leave: no `TRAP` catches it and the thread dies. Every caller must know the
//! destination has room *before* it calls, which is what `symbian-core` does.

use crate::des::{KMASK_DES_LENGTH_16, TDesC16};

/// The opaque `TDes16` a modifying member function is called on: passed as `this`, i.e.
/// argument 0. Only ever seen behind a pointer.
#[repr(C)]
pub struct TDes16 {
    _private: [u8; 0],
}

/// Caller-owned storage a `TPtr16` is built into by euser's own constructor.
///
/// `sizeof(TPtr16) == 12` and `__alignof__(TPtr16) == 4`, measured by compiling
/// `return sizeof(TPtr16);` with the recorded GCCE argv and reading the immediate
/// (`movs r0, #12` / `movs r0, #4`). Nothing here writes a header word: euser builds
/// the descriptor, exactly as [`crate::des8`] does for the 8-bit family, so the type
/// nibble of a `TPtr16` is never guessed at.
#[repr(C, align(4))]
pub struct TPtr16Storage {
    words: [u32; 3],
}

impl TPtr16Storage {
    /// Zeroed storage, ready for euser's constructor to build a descriptor into.
    pub const fn zeroed() -> Self {
        Self { words: [0; 3] }
    }

    /// The `TDes16&` a filling export expects. A `TPtr16` is a `TDes16` is a `TDesC16`,
    /// and all three begin at the header word.
    pub const fn as_tdes16(&mut self) -> *mut TDes16 {
        (self as *mut Self).cast()
    }

    /// The length in code units, from the header word's documented low 28 bits.
    pub const fn length(&self) -> usize {
        (self.words[0] & KMASK_DES_LENGTH_16) as usize
    }
}

unsafe extern "C" {
    /// `00000ee0 T _ZN6TDes164CopyERK7TDesC16` — `TDes16::Copy(const TDesC16&)`.
    /// Replaces the contents. Panics `USER 11` if `src` is longer than `MaxLength()`.
    #[link_name = "_ZN6TDes164CopyERK7TDesC16"]
    pub fn TDes16_Copy(this: *mut TDes16, src: *const TDesC16);

    /// `00000f1c T _ZN6TDes166AppendERK7TDesC16` — `TDes16::Append(const TDesC16&)`.
    /// Panics `USER 11` on overflow.
    #[link_name = "_ZN6TDes166AppendERK7TDesC16"]
    pub fn TDes16_Append(this: *mut TDes16, src: *const TDesC16);

    /// `00000f18 T _ZN6TDes166AppendEPKti` — `TDes16::Append(const TUint16* aBuf, TInt
    /// aLength)`: `aLength` code units from `aBuf`. Panics `USER 11` on overflow.
    /// Experiment 106 compiled `d->Append(p, n)` with symdev's GCCE argv: `push {r4, lr};
    /// blx Append; pop` — no shuffle, so `this`, `aBuf`, `aLength` are r0, r1, r2.
    #[link_name = "_ZN6TDes166AppendEPKti"]
    pub fn TDes16_AppendUnits(this: *mut TDes16, units: *const u16, length: i32);

    /// `00000f5c T _ZN6TDes169AppendNumEx` — `TDes16::AppendNum(TInt64)`: the signed
    /// decimal integer, formatted by euser and therefore free of `core::fmt`.
    ///
    /// `e32des16.h` has no `AppendNum(TInt)`; the integer overloads are this one and
    /// `AppendNum(TUint64, TRadix)`. Panics `USER 11` on overflow.
    #[link_name = "_ZN6TDes169AppendNumEx"]
    pub fn TDes16_AppendNum(this: *mut TDes16, value: i64);

    /// `00000ec8 T _ZN6TDes163NumEx` — `TDes16::Num(TInt64)`: `AppendNum` after `Zero`.
    #[link_name = "_ZN6TDes163NumEx"]
    pub fn TDes16_Num(this: *mut TDes16, value: i64);

    /// `00001008 T _ZN6TPtr16C1EPtii` — `TPtr16::TPtr16(TUint16* aBuf, TInt aLength,
    /// TInt aMaxLength)`, euser.dso: a writable view over storage the caller owns,
    /// built in place so that the descriptor header comes from euser.
    #[link_name = "_ZN6TPtr16C1EPtii"]
    pub fn TPtr16_ctor(this: *mut TPtr16Storage, buf: *mut u16, length: i32, max_length: i32);
}
