//! `Request`: one file-server call that names a path, made through one body
//! (experiment 105).
//!
//! Every `RFs` call that takes a path, and the `RFile` calls that open one, go through
//! [`Request::on`]: the path is encoded into its `TFileName` there, once, in that frame,
//! and the call itself is handed in. Before, each caller built its own
//! `Buf16<KMaxFileName>`, returned it by value (a 516-byte copy) and wrapped its call in
//! its own inlined copy of the session logic, so an image paid for the path and the
//! session once per call site rather than once.
//!
//! The call is a `&mut dyn FnMut`, not a variant of an enum the body matches on: a
//! `match` would link every file-server call into every image that makes any of them
//! (measured, experiment 105), where a passed-in call links only the ones an image uses.
use symbian_sys::des::TDesC16;
use symbian_sys::efsrv::{RFs, RFs_Rename};

use super::MAX_FILE_NAME;
use crate::des::{Buf16, DesC16};
use crate::error::{Result, check};

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

/// The Symbian call a [`Request`] makes: the session and the path in, the `TInt` out.
///
/// A trait of its own rather than `dyn FnMut`: `FnMut`'s vtable also holds a
/// `call_once` shim, a second copy of every closure's body (measured, experiment 105).
pub(crate) trait Call {
    /// Makes the call.
    fn call(&mut self, fs: *mut RFs, path: *const TDesC16) -> i32;
}

impl<F: FnMut(*mut RFs, *const TDesC16) -> i32> Call for F {
    fn call(&mut self, fs: *mut RFs, path: *const TDesC16) -> i32 {
        self(fs, path)
    }
}

/// A file-server call on one path: the call, and what the path needs first.
pub(crate) struct Request<'a> {
    /// Appended to the path: nothing, or what a directory call needs.
    tail: &'static str,
    call: &'a mut dyn Call,
}

impl<'a> Request<'a> {
    /// `call` on the path as it is given.
    ///
    /// The bound is `FnMut` rather than [`Call`] so that a closure written at the call
    /// gets its parameter types from here.
    pub(crate) fn new(call: &'a mut impl FnMut(*mut RFs, *const TDesC16) -> i32) -> Self {
        Self { tail: "", call }
    }

    /// `path` names a directory: a backslash is added when it does not end with one.
    /// `RFs::MkDirAll` takes the last component as a file name without it.
    pub(crate) fn of_directory(mut self, path: &str) -> Self {
        self.tail = if path.ends_with('\\') { "" } else { "\\" };
        self
    }

    /// Every entry of the directory `path` names: the backslash when it is missing,
    /// then `*`. `f32file.h` says a path to search "should always end with a backslash
    /// character. When trailing backslash is not present then it is considered as file".
    pub(crate) fn of_every_entry(mut self, path: &str) -> Self {
        self.tail = if path.ends_with('\\') { "*" } else { "\\*" };
        self
    }

    /// Makes the call on the session `fs`, with `path` encoded into a `TFileName`.
    ///
    /// `KErrOverflow` if the path, with what a directory call adds, is longer than
    /// `KMaxFileName`; otherwise the `TInt` the call returned, as a `Result`.
    ///
    /// # Safety
    ///
    /// `fs` is a connected `RFs` that nothing else uses for the length of the call.
    pub(crate) unsafe fn on(self, fs: *mut RFs, path: &str) -> Result<i32> {
        // Built where it is used: `path_of` returns the 516-byte buffer by value, and
        // the copy is a `memcpy` at every caller.
        let mut name: Buf16<MAX_FILE_NAME> = Buf16::new();
        name.push_str(path)?;
        name.push_str(self.tail)?;
        check(self.call.call(fs, name.as_tdesc16()))
    }
}

/// `RFs::Rename` of the path in `from` to `to`, encoded here in the frame that uses it,
/// as [`Request::on`] encodes the first path: a `TFileName` returned by value is a
/// 516-byte copy. The `TInt` of the call, or `KErrOverflow` for a `to` that does not fit.
///
/// # Safety
///
/// `fs` is a connected session nothing else uses for the call; `from` a live
/// descriptor.
pub(crate) unsafe fn rename(fs: *mut RFs, from: *const TDesC16, to: &str) -> i32 {
    let mut to_name: Buf16<MAX_FILE_NAME> = Buf16::new();
    match to_name.push_str(to) {
        // SAFETY: `this` first per the observed member ABI, both descriptors only
        // read. Non-leaving; every failure is the returned `TInt`.
        Ok(()) => unsafe { RFs_Rename(fs, from, to_name.as_tdesc16()) },
        Err(e) => e.code(),
    }
}
