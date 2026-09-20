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
    /// Returns null on failure. Cell alignment: not observed yet (experiment 66).
    #[link_name = "_ZN4User5AllocEi"]
    pub fn User_Alloc(size: i32) -> *mut u8;

    /// `00000a0c T _ZN4User4FreeEPv` — `User::Free(TAny*)`.
    #[link_name = "_ZN4User4FreeEPv"]
    pub fn User_Free(cell: *mut u8);

    /// `00000a3c T _ZN4User7ReAllocEPvii` — `User::ReAlloc(TAny*, TInt, TInt)`; the third
    /// argument is the mode (0 = default). Semantics beyond that: not observed yet
    /// (experiment 66).
    #[link_name = "_ZN4User7ReAllocEPvii"]
    pub fn User_ReAlloc(cell: *mut u8, size: i32, mode: i32) -> *mut u8;
}
