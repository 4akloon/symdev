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

use crate::des::TDesC16;

/// The opaque `TDes16` a modifying member function is called on: passed as `this`, i.e.
/// argument 0. Only ever seen behind a pointer.
#[repr(C)]
pub struct TDes16 {
    _private: [u8; 0],
}

/// `TChar` is a class wrapping one `TUint`: 4 bytes, one register under the EABI.
pub type TChar = u32;

unsafe extern "C" {
    /// `00000ee0 T _ZN6TDes164CopyERK7TDesC16` — `TDes16::Copy(const TDesC16&)`.
    /// Replaces the contents. Panics `USER 11` if `src` is longer than `MaxLength()`.
    #[link_name = "_ZN6TDes164CopyERK7TDesC16"]
    pub fn TDes16_Copy(this: *mut TDes16, src: *const TDesC16);

    /// `00000f1c T _ZN6TDes166AppendERK7TDesC16` — `TDes16::Append(const TDesC16&)`.
    /// Panics `USER 11` on overflow.
    #[link_name = "_ZN6TDes166AppendERK7TDesC16"]
    pub fn TDes16_Append(this: *mut TDes16, src: *const TDesC16);

    /// `00000f14 T _ZN6TDes166AppendE5TChar` — `TDes16::Append(TChar)`. One code unit
    /// for anything in the basic multilingual plane. Panics `USER 11` on overflow.
    #[link_name = "_ZN6TDes166AppendE5TChar"]
    pub fn TDes16_AppendChar(this: *mut TDes16, c: TChar);

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
}
