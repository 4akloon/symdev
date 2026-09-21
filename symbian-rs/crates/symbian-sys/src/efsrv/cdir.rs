//! `RFs::GetDir` and `class CDir` (`f32file.h` lines 1597 and 1740): a directory read
//! into memory as an array of [`TEntry`].
//!
//! # Why `GetDir` is declared here and `delete` is not
//!
//! `RFs::GetDir` returns a `TInt`, and its leaving counterparts — `RFs::GetDirL` and
//! `DoGetDirL` (`f32file.h` lines 1827-1830) — are **private**. efsrv runs the trap
//! harness itself and hands the caller an error code, so the step-70 rule puts this
//! call in Rust and not in the C++ shim. `CDir::NewL` and `AddL` are protected and
//! `friend class RFs`, so nothing outside efsrv can reach a leaving entry point on the
//! type at all.
//!
//! What does need the shim is destroying the answer: `~CDir()` is `IMPORT_C virtual`,
//! and a virtual is rule 3 of `shims/common/symrs_shim.h`. `delete aDir` goes through
//! the vtable, so it is written in C++ as [`crate::shim::symrs_f32_dir_delete`] rather
//! than guessed at from `_ZN4CDirD0Ev`, which would be assuming the dynamic type.
//!
//! Everything read back — `Count`, `operator[]` — is a non-virtual `const` member
//! taking only `this`, so it is called directly with the ABI experiment 78 observed.

use super::entry::TEntry;
use crate::des::TDesC16;

/// `KEntryAttMatchMask` (`f32file.h` line 364): `KEntryAttHidden | KEntryAttSystem |
/// KEntryAttDir`, the mask that makes `GetDir` match **everything**.
///
/// It has to be spelled out because `KEntryAttNormal` is 0 and, as the header says in
/// as many words, "matches all entry types except directories, hidden and system
/// entries" — which is not what `std::fs::read_dir` promises.
pub const KENTRY_ATT_MATCH_MASK: u32 = 0x0002 | 0x0004 | 0x0010;

/// `ESortNone` (`TEntryKey`, `f32file.h` line 728): no sorting.
///
/// `std::fs::read_dir` promises no order, and sorting is work the file server would do
/// for nothing.
pub const ESORT_NONE: u32 = 0;

/// The `CDir` `RFs::GetDir` allocates; only ever seen behind a pointer, because it is a
/// `CBase` with a `CArrayPakFlat<TEntry>` inside it and no part of that is a C type.
#[repr(C)]
pub struct CDir {
    _private: [u8; 0],
}

unsafe extern "C" {
    /// `000003b4 T _ZNK3RFs6GetDirERK7TDesC16jjRP4CDir` — `RFs::GetDir(const TDesC16&
    /// aName, TUint anEntryAttMask, TUint anEntrySortKey, CDir*& anEntryList) const`.
    ///
    /// On `KErrNone` the caller owns the `CDir` and must destroy it with
    /// [`crate::shim::symrs_f32_dir_delete`]. `aName` names a **directory**, and
    /// `f32file.h` is explicit that the path "should always end with a backslash
    /// character. When trailing backslash is not present then it is considered as
    /// file."
    #[link_name = "_ZNK3RFs6GetDirERK7TDesC16jjRP4CDir"]
    pub fn RFs_GetDir(
        this: *const super::rfs::RFs,
        name: *const TDesC16,
        att_mask: u32,
        sort_key: u32,
        entry_list: *mut *mut CDir,
    ) -> i32;

    /// `000003d8 T _ZNK4CDir5CountEv` — `CDir::Count() const`: how many entries.
    #[link_name = "_ZNK4CDir5CountEv"]
    pub fn CDir_Count(this: *const CDir) -> i32;

    /// `000003dc T _ZNK4CDirixEi` — `const TEntry& CDir::operator[](TInt) const`.
    ///
    /// A reference return is a pointer under the EABI, so this is a C signature. The
    /// index must be below `Count()`: `CArrayPakFlat` panics on one that is not, and a
    /// panic is not a leave, so no `TRAP` anywhere could catch it.
    #[link_name = "_ZNK4CDirixEi"]
    pub fn CDir_At(this: *const CDir, index: i32) -> *const TEntry;
}
