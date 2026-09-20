//! `efsrv.dll` exports (`class RFs`), from `nm -D
//! epoc32/release/armv5/lib/efsrv.dso`. Non-static member functions, called with `this`
//! as argument 0 (the member ABI observed in [`crate::des16`]).
//!
//! None of these leaves: `f32file.h` declares no leaving member on `RFs` at all (`grep
//! -E "IMPORT_C.*[A-Za-z]L\("` over the header finds nothing), and every one of them
//! reports its failure as a `TInt`. That is why they are here and not in the shim.

use crate::des::TDesC16;

/// `RFs` is `RSessionBase` is `RHandleBase`: one `TInt iHandle`, and nothing else —
/// `sizeof(RFs) == 4`, measured by compiling `return sizeof(RFs);` with the recorded
/// GCCE argv and reading the immediate (`movs r0, #4`). A default-constructed `RFs` has
/// `iHandle == 0`.
#[repr(C)]
pub struct RFs {
    pub handle: i32,
}

/// `KFileServerDefaultMessageSlots`, the default argument of `RFs::Connect`. Observed in
/// the same probe: `do_connect` emits `movs r1, #1; negs r1, r1` before the call.
pub const KFILE_SERVER_DEFAULT_MESSAGE_SLOTS: i32 = -1;

unsafe extern "C" {
    /// `0000010c T _ZN3RFs7ConnectEi` — `RFs::Connect(TInt aMessageSlots)`. Opens the
    /// session with the file server; returns a system-wide error code.
    #[link_name = "_ZN3RFs7ConnectEi"]
    pub fn RFs_Connect(this: *mut RFs, message_slots: i32) -> i32;

    /// `00000114 T _ZN3RFs8MkDirAllERK7TDesC16` — `RFs::MkDirAll(const TDesC16&)`:
    /// creates every directory of the path that does not exist yet. Non-leaving, and the
    /// direct counterpart of the trapped `BaflUtils::EnsurePathExistsL` in the shim.
    #[link_name = "_ZN3RFs8MkDirAllERK7TDesC16"]
    pub fn RFs_MkDirAll(this: *mut RFs, path: *const TDesC16) -> i32;

    /// `000000f8 T _ZN3RFs5MkDirERK7TDesC16` — `RFs::MkDir(const TDesC16&)`: one
    /// directory, whose parent must exist.
    #[link_name = "_ZN3RFs5MkDirERK7TDesC16"]
    pub fn RFs_MkDir(this: *mut RFs, path: *const TDesC16) -> i32;
}
