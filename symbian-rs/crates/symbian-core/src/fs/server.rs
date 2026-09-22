//! `FileServer`: a session with the Symbian file server (`RFs`), and the place where
//! the two halves of the shim rule stand side by side (design spec §7, steps 70–71).
//!
//! Every `RFs` member called here is **non-leaving** and is called straight from Rust
//! with `this` as argument 0, with one deliberate exception:
//! [`FileServer::ensure_path_exists`] is `BaflUtils::EnsurePathExistsL` from bafl.dso,
//! which **leaves** and therefore goes through the C++ `TRAP` shim. Calling that one
//! directly would end the process without a word the first time a path was wrong
//! (experiment 76).
//!
//! Both report failure the same way: a `SymbianError` holding the exact `TInt`. Both
//! take a `&str`, not a descriptor: an application using this SDK should not have to
//! name a Symbian type to open a path.
use symbian_sys::efsrv::{KFILE_SERVER_DEFAULT_MESSAGE_SLOTS, RFs, RFs_Att, RFs_Connect};
use symbian_sys::euser::{RHandleBase, RHandleBase_Close};
use symbian_sys::shim::symrs_bafl_ensure_path_exists;

use super::entry::Entry;
use super::path_of;
use super::request::Request;
use crate::des::DesC16;
use crate::error::{Result, check};

/// An open session with the file server.
///
/// The handle is closed when the value is dropped. It is neither `Send` nor `Sync`: a
/// session belongs to the thread that opened it, as every Symbian kernel handle does.
pub struct FileServer {
    fs: RFs,
}

impl FileServer {
    /// Opens the session (`RFs::Connect`).
    pub fn connect() -> Result<Self> {
        // A default-constructed `RFs` is `iHandle == 0`; `Connect` fills it in.
        let mut server = Self {
            fs: RFs { handle: 0 },
        };
        // SAFETY: `RFs` is the observed one-word layout (`sizeof(RFs) == 4`) and is
        // passed as `this`, argument 0, per the observed member ABI. `Connect` is
        // non-leaving and reports every failure as the returned `TInt`.
        let code = unsafe { RFs_Connect(&mut server.fs, KFILE_SERVER_DEFAULT_MESSAGE_SLOTS) };
        check(code)?;
        Ok(server)
    }

    /// The `RFs*` an `RFile` member takes as its own argument. Crate-internal: a
    /// session is something an application holds, not something it passes around.
    pub(crate) fn as_rfs(&mut self) -> *mut RFs {
        &mut self.fs
    }

    /// Creates every directory of `path` that does not exist yet (`RFs::MkDirAll`).
    ///
    /// `path` is a full file name: the last component is taken as the file and is not
    /// created. An existing path is `KErrAlreadyExists`, not success.
    ///
    /// **No shim.** The call cannot leave, so Rust makes it itself.
    pub fn make_dir_all(&mut self, path: &str) -> Result<()> {
        // SAFETY: the session is `self`, borrowed mutably for the call; the request
        // carries no pointer. Non-leaving, so no C++ exception can cross this frame.
        unsafe { Request::MakeDirAll.on(&mut self.fs, path) }.map(|_| ())
    }

    /// Creates every missing directory of `path`, treating one that is already there as
    /// success (`BaflUtils::EnsurePathExistsL`, **through the shim**).
    ///
    /// The SDK call leaves with the file server's own error — a drive that is not
    /// mounted, a path that cannot be written — and the shim's `TRAP` turns that into
    /// this `Err` while the process carries on.
    pub fn ensure_path_exists(&mut self, path: &str) -> Result<()> {
        let path = path_of(path)?;
        // SAFETY: the shim takes `RFs*` and `const TDesC16*`, both borrowed for the
        // call, and is a complete `TRAP` unit: it returns `KErrNone`, the leave code or
        // `KErrArgument`, and never lets an exception out. Nothing unwinds into Rust.
        let code = unsafe { symrs_bafl_ensure_path_exists(&mut self.fs, path.as_tdesc16()) };
        check(code).map(|_| ())
    }

    /// Removes one file (`RFs::Delete`).
    ///
    /// `KErrInUse` if it is open and `KErrAccessDenied` for a directory: the file
    /// server has no unlink-while-open.
    pub fn delete(&mut self, path: &str) -> Result<()> {
        // SAFETY: the session is `self`, borrowed mutably; the request carries no pointer.
        unsafe { Request::Delete.on(&mut self.fs, path) }.map(|_| ())
    }

    /// Renames a file or directory (`RFs::Rename`).
    ///
    /// `KErrAlreadyExists` if `to` is taken — Symbian does not replace silently the way
    /// POSIX `rename` does.
    pub fn rename(&mut self, from: &str, to: &str) -> Result<()> {
        // SAFETY: the session is `self`, borrowed mutably; `to` is a `&str`, not a pointer.
        unsafe { Request::Rename(to).on(&mut self.fs, from) }.map(|_| ())
    }

    /// The `KEntryAtt*` bits of an existing entry (`RFs::Att`).
    pub fn attributes(&self, path: &str) -> Result<u32> {
        let path = path_of(path)?;
        let mut att = 0u32;
        // SAFETY: a `const` member, so `this` is shared; the descriptor is borrowed and
        // only read, and `att` is a live `TUint` the call writes once. Non-leaving.
        let code = unsafe { RFs_Att(&self.fs, path.as_tdesc16(), &mut att) };
        check(code)?;
        Ok(att)
    }

    /// Everything the file server knows about one entry (`RFs::Entry`).
    pub fn entry(&self, path: &str) -> Result<Entry> {
        let mut entry = Entry::new();
        // SAFETY: `RFs::Entry` is a `const` member, so the `*mut` is only ever read
        // through; the `TEntry` is a real one, built by euser's own exported
        // constructor in storage of the measured size and alignment, and borrowed
        // mutably for the call. Non-leaving.
        let fs = (&raw const self.fs).cast_mut();
        unsafe { Request::Entry(entry.as_tentry()).on(fs, path) }?;
        Ok(entry)
    }
}

impl Drop for FileServer {
    fn drop(&mut self) {
        // SAFETY: `RFs` and `RHandleBase` are the same one-word layout, and `RFs::Close`
        // compiles to exactly this call (observed). It is non-leaving and safe on a
        // handle that is already 0.
        unsafe { RHandleBase_Close((&mut self.fs as *mut RFs).cast::<RHandleBase>()) }
    }
}
