//! `class RFs`, the file server session, from `nm -D
//! epoc32/release/armv5/lib/efsrv.dso`. Non-static member functions, called with `this`
//! as argument 0 (the member ABI observed in [`crate::des16`]).
//!
//! None of these leaves: `f32file.h` declares no leaving member on `RFs` at all (`grep
//! -aE "IMPORT_C.*[A-Za-z]L\("` over the header finds nothing on this class), and every
//! one of them reports its failure as a `TInt`. That is why they are here and not in
//! the shim.

use super::entry::TEntry;
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

    /// `00000100 T _ZN3RFs6DeleteERK7TDesC16` — `RFs::Delete(const TDesC16& aName)`:
    /// removes one file. `KErrInUse` if it is open, `KErrAccessDenied` for a directory.
    #[link_name = "_ZN3RFs6DeleteERK7TDesC16"]
    pub fn RFs_Delete(this: *mut RFs, name: *const TDesC16) -> i32;

    /// `00000104 T _ZN3RFs6RenameERK7TDesC16S2_` — `RFs::Rename(const TDesC16& anOld,
    /// const TDesC16& aNew)`: renames a file or directory. `KErrAlreadyExists` if the
    /// new name is taken — unlike POSIX `rename`, which replaces silently.
    #[link_name = "_ZN3RFs6RenameERK7TDesC16S2_"]
    pub fn RFs_Rename(this: *mut RFs, old_name: *const TDesC16, new_name: *const TDesC16) -> i32;

    /// `00000398 T _ZNK3RFs3AttERK7TDesC16Rj` — `RFs::Att(const TDesC16& aName, TUint&
    /// aAttValue) const`: the `KEntryAtt*` bits of an existing entry.
    #[link_name = "_ZNK3RFs3AttERK7TDesC16Rj"]
    pub fn RFs_Att(this: *const RFs, name: *const TDesC16, att: *mut u32) -> i32;

    /// `000003a0 T _ZNK3RFs5EntryERK7TDesC16R6TEntry` — `RFs::Entry(const TDesC16&
    /// aName, TEntry& anEntry) const`: the attributes, size, modification time and UID
    /// type of one entry, into a caller-owned [`TEntry`].
    #[link_name = "_ZNK3RFs5EntryERK7TDesC16R6TEntry"]
    pub fn RFs_Entry(this: *const RFs, name: *const TDesC16, entry: *mut TEntry) -> i32;
}
