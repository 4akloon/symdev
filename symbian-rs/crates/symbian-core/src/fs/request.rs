//! `Request`: one file-server call that names a path, made in one body (experiment 105).
//!
//! Every `RFs` call that takes a path, and the three `RFile` calls that open one, go
//! through [`Request::on`]: the path is encoded into its `TFileName` there, once, in that
//! frame, and the call is one arm of a `match`. Before, each caller built its own
//! `Buf16<KMaxFileName>`, returned it by value (a 516-byte copy) and wrapped its call in
//! its own inlined copy of the session logic, so an image paid for the path and the
//! session once per call site rather than once.
use symbian_sys::des::TDesC16;
use symbian_sys::efsrv::{
    CDir, ESORT_NONE, KENTRY_ATT_MATCH_MASK, RFile, RFile_Create, RFile_Open, RFile_Replace, RFs,
    RFs_Delete, RFs_Entry, RFs_GetDir, RFs_MkDirAll, RFs_Rename, TEntry,
};

use super::path_of;
use crate::des::DesC16;
use crate::error::{Result, check};
use crate::{ErrorKind, SymbianError};

/// Which `RFile` call opens a file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Opening {
    /// `RFile::Open`: the file must exist.
    Existing,
    /// `RFile::Create`: the file must not exist.
    New,
    /// `RFile::Replace`: create it, or truncate the one that is there.
    Replace,
    /// `RFile::Open`, then `RFile::Create` if that said `KErrNotFound`. Symbian has no
    /// create-if-missing-keep-contents call, so this is the pair of them, on one path.
    OpenOrCreate,
}

/// A file-server call on one path, and whatever else the call takes.
pub(crate) enum Request<'a> {
    /// `RFs::MkDirAll`: the last component is a file name and is not created.
    MakeDirAll,
    /// `RFs::MkDirAll` of the directory the path names: a separator is added when the
    /// path does not end with one, so the last component is created too.
    MakeDir,
    /// `RFs::Delete`.
    Delete,
    /// `RFs::Rename` to the second path.
    Rename(&'a str),
    /// `RFs::Entry` into a `TEntry` built by its constructor.
    Entry(*mut TEntry),
    /// `RFs::GetDir` of every entry of the directory the path names: a separator is
    /// added when missing, then the `*` wildcard (see [`super::Dir::read`]).
    GetDir(*mut *mut CDir),
    /// One of the `RFile` opening calls, into a closed `RFile`, with a `TFileMode`.
    Open(*mut RFile, u32, Opening),
}

impl Request<'_> {
    /// Makes the call on the session `fs`, with `path` encoded into a `TFileName`.
    ///
    /// `KErrOverflow` if the path, with the separator and wildcard a directory call adds,
    /// is longer than `KMaxFileName`; otherwise the `TInt` the file server returned.
    ///
    /// # Safety
    ///
    /// `fs` is a connected `RFs` that nothing else uses for the length of the call, and
    /// every pointer the request carries is live and valid for what the call writes: a
    /// `TEntry` built by its constructor, a `CDir*` slot, a closed `RFile`.
    #[inline(never)]
    pub(crate) unsafe fn on(self, fs: *mut RFs, path: &str) -> Result<i32> {
        let mut name = path_of(path)?;
        if matches!(self, Self::MakeDir | Self::GetDir(_)) && !path.ends_with('\\') {
            name.push('\\')?;
        }
        if matches!(self, Self::GetDir(_)) {
            name.push('*')?;
        }
        let des = name.as_tdesc16();
        // SAFETY: `this` in argument 0 per the observed member ABI; the descriptors are
        // borrowed for the call and only read; the other pointers are the caller's
        // promise above. Every call here is non-leaving and reports failure as the
        // returned `TInt` (`f32file.h`; `GetDir` traps its private `GetDirL` itself and
        // uses the cleanup stack every Rust entry point installs, experiment 97).
        let code = unsafe {
            match self {
                Self::MakeDirAll | Self::MakeDir => RFs_MkDirAll(fs, des),
                Self::Delete => RFs_Delete(fs, des),
                Self::Rename(to) => RFs_Rename(fs, des, path_of(to)?.as_tdesc16()),
                Self::Entry(entry) => RFs_Entry(fs, des, entry),
                Self::GetDir(dir) => RFs_GetDir(fs, des, KENTRY_ATT_MATCH_MASK, ESORT_NONE, dir),
                Self::Open(file, mode, how) => open(file, fs, des, mode, how),
            }
        };
        check(code)
    }
}

/// The opening call, or the pair of them for [`Opening::OpenOrCreate`].
///
/// # Safety
///
/// As [`Request::on`]: `file` is a closed `RFile`, `fs` a connected session, `path` a
/// live descriptor.
unsafe fn open(file: *mut RFile, fs: *mut RFs, path: *const TDesC16, mode: u32, how: Opening) -> i32 {
    let call = match how {
        Opening::Existing | Opening::OpenOrCreate => RFile_Open,
        Opening::New => RFile_Create,
        Opening::Replace => RFile_Replace,
    };
    // SAFETY: the three calls take the same arguments and none of them leaves.
    let code = unsafe { call(file, fs, path, mode) };
    if how == Opening::OpenOrCreate && code == SymbianError::of(ErrorKind::NotFound).code() {
        // What a failed `Open` leaves in the handle is not documented, so `Create` gets
        // a closed one, as it did when the two calls had a handle each.
        // SAFETY: `file` is live (the caller's promise); the same call as above.
        return unsafe {
            file.write(RFile {
                handle: 0,
                sub_session_handle: 0,
            });
            RFile_Create(file, fs, path, mode)
        };
    }
    code
}
