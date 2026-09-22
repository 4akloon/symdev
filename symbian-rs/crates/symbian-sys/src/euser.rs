//! `euser.dll` exports (`class User`), from `nm -D epoc32/release/armv5/lib/euser.dso`.
//! All are static member functions: no `this`, plain EABI (experiment 65a). Each name
//! below is the unversioned part of what `nm` printed; the linker's `--default-symver`
//! binds it to the DSO's versioned export as it does for C++ objects.

use crate::des::TDesC16;

/// `TTimeIntervalMicroSeconds32` is a 4-byte class passed by value: one register under
/// the EABI (experiment 65a: a 5-second `User::After` was honoured).
pub type TTimeIntervalMicroSeconds32 = i32;

unsafe extern "C" {
    /// `00000a6c T _ZN4User9InfoPrintERK7TDesC16` — `User::InfoPrint(const TDesC16&)`.
    #[link_name = "_ZN4User9InfoPrintERK7TDesC16"]
    pub fn User_InfoPrint(des: *const TDesC16) -> i32;

    /// `00000a10 T _ZN4User5AfterE27TTimeIntervalMicroSeconds32` —
    /// `User::After(TTimeIntervalMicroSeconds32)`.
    #[link_name = "_ZN4User5AfterE27TTimeIntervalMicroSeconds32"]
    pub fn User_After(interval: TTimeIntervalMicroSeconds32);

    /// `00000a00 T _ZN4User4ExitEi` — `User::Exit(TInt)`; ends the process.
    #[link_name = "_ZN4User4ExitEi"]
    pub fn User_Exit(reason: i32) -> !;

    /// `00000a24 T _ZN4User5PanicERK7TDesC16i` — `User::Panic(const TDesC16&, TInt)`.
    #[link_name = "_ZN4User5PanicERK7TDesC16i"]
    pub fn User_Panic(category: *const TDesC16, reason: i32) -> !;

    /// `00000a14 T _ZN4User5AllocEi` — `User::Alloc(TInt)`, the non-leaving variant.
    /// Returns null on failure. The cell is 8-byte aligned and its size is rounded up to
    /// a multiple of 8 with a 36-byte minimum payload (experiment 68, measured on the
    /// ROM's own heap inside EKA2L1).
    #[link_name = "_ZN4User5AllocEi"]
    pub fn User_Alloc(size: i32) -> *mut u8;

    /// `00000a2c T _ZN4User6AllocZEi` — `User::AllocZ(TInt)`: `Alloc` with the cell
    /// zero-filled.
    #[link_name = "_ZN4User6AllocZEi"]
    pub fn User_AllocZ(size: i32) -> *mut u8;

    /// `00000a4c T _ZN4User8AllocLenEPKv` — `User::AllocLen(const TAny*)`: the usable
    /// length of a cell, which is how experiment 68 measured the heap's granularity.
    #[link_name = "_ZN4User8AllocLenEPKv"]
    pub fn User_AllocLen(cell: *const u8) -> i32;

    /// `00000980 T _ZN4User15CountAllocCellsEv` — `User::CountAllocCells()`
    /// (`e32std.h:4486`): how many cells are allocated on the current thread's heap.
    /// Non-leaving. It is how a test proves a code path allocates nothing, by counting
    /// either side of it.
    #[link_name = "_ZN4User15CountAllocCellsEv"]
    pub fn User_CountAllocCells() -> i32;

    /// `00000a0c T _ZN4User4FreeEPv` — `User::Free(TAny*)`.
    #[link_name = "_ZN4User4FreeEPv"]
    pub fn User_Free(cell: *mut u8);

    /// `00000a54 T _ZN4User8LanguageEv` — `User::Language()`, declared
    /// `IMPORT_C static TLanguage Language();` at `e32std.h` line 4531.
    ///
    /// `TLanguage` is the enum of `e32const.h` line 1439 (`ELangTest = 0`,
    /// `ELangEnglish = 1`, … `ELangNone = 0xFFFF`). It is a plain C enum, and the
    /// target JSON's `c-enum-min-bits: 32` says it comes back in r0 as a 32-bit value;
    /// the SDK's own comment on `ELangNone` caps the range at 1023 languages.
    #[link_name = "_ZN4User8LanguageEv"]
    pub fn User_Language() -> i32;

    /// `00000a78 T _ZN4User9LockedIncERi` — `User::LockedInc(TInt&)`, declared in
    /// `e32std.h` line 4519 under the comment `// Atomic operations`.
    ///
    /// **The only genuinely atomic primitive euser exports** (with its three
    /// neighbours): an unconditional +1 that returns the **old** value. Observed on
    /// this ROM in EKA2L1: `0→1 r=0`, `5→6 r=5`, `−1→0 r=−1`, and two threads doing
    /// 20000 each gave exactly 40000 while a hand-rolled read-modify-write beside it
    /// lost half its updates (experiment 72).
    ///
    /// It needs no lock and no initialisation, which is what makes it the right thing
    /// to bootstrap a lock with. The headers state **no memory ordering** for it and
    /// nothing on this host can settle that (UNKNOWN, experiment 72).
    #[link_name = "_ZN4User9LockedIncERi"]
    pub fn User_LockedInc(value: *mut i32) -> i32;

    /// `00000a74 T _ZN4User9LockedDecERi` — `User::LockedDec(TInt&)`: the same, −1.
    #[link_name = "_ZN4User9LockedDecERi"]
    pub fn User_LockedDec(value: *mut i32) -> i32;

    /// `000009b4 T _ZN4User17CommandLineLengthEv` — `User::CommandLineLength()`
    /// (`e32std.h` line 4569): how many code units the process's command line is.
    #[link_name = "_ZN4User17CommandLineLengthEv"]
    pub fn User_CommandLineLength() -> i32;

    /// `00000918 T _ZN4User11CommandLineER6TDes16` — `User::CommandLine(TDes16&)`
    /// (`e32std.h` line 4570): the command line the creator passed to
    /// `RProcess::Create`, copied into the caller's descriptor.
    ///
    /// **It is `User::`, not `RProcess::`**: this SDK's `euser.dso` exports no
    /// `RProcess::CommandLine` at all. It returns `void` and there is no error path; a
    /// destination shorter than [`User_CommandLineLength`] would panic `USER 11`, which
    /// no `TRAP` catches, so the caller must size the buffer first.
    #[link_name = "_ZN4User11CommandLineER6TDes16"]
    pub fn User_CommandLine(command: *mut crate::des16::TDes16);

    /// `00000a3c T _ZN4User7ReAllocEPvii` — `User::ReAlloc(TAny*, TInt, TInt)`; the third
    /// argument is the mode (0 = default: the cell may move, the common prefix is kept,
    /// failure returns null and leaves the old cell alone — experiment 68).
    #[link_name = "_ZN4User7ReAllocEPvii"]
    pub fn User_ReAlloc(cell: *mut u8, size: i32, mode: i32) -> *mut u8;
}

/// `RHandleBase`: the base of every kernel handle (`RFs`, `RFile`, …). One `TInt
/// iHandle`, and the only member the SDK needs here.
#[repr(C)]
pub struct RHandleBase {
    pub handle: i32,
}

unsafe extern "C" {
    /// `000001dc T _ZN11RHandleBase5CloseEv` — `RHandleBase::Close()`, which is what
    /// `RFs::Close()` compiles to (observed: `do_close(RFs*)` emits a bare
    /// `bl _ZN11RHandleBase5CloseEv`). Non-leaving, and safe on a handle that was never
    /// opened: a default-constructed handle is 0.
    #[link_name = "_ZN11RHandleBase5CloseEv"]
    pub fn RHandleBase_Close(this: *mut RHandleBase);
}
