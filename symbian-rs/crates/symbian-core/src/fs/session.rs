//! The one file-server session a process has, so that a caller can write
//! `fs::File::open(path)` without first naming an `RFs`.
//!
//! Symbian's own convention is one `RFs` per thread, connected once and kept for the
//! life of the program: a session is an IPC connection to a server, not a cheap value.
//! [`with_session`] connects on first use and hands the session to a closure; every
//! call that needs `RFs` — opening a file, deleting one, asking for an entry — does its
//! work inside that closure, and an `RFile` needs no session afterwards because it
//! carries its own sub-session handle.
//!
//! The session is deliberately never closed: it lives until the process ends, and the
//! kernel closes every handle then. Dropping it while an `RFile` opened from it is
//! still alive would be the bug, not the leak.
use core::cell::{Cell, UnsafeCell};

use super::request::Request;
use super::server::FileServer;
use crate::error::Result;
use crate::{ErrorKind, SymbianError};

/// The process's session, plus the flag that keeps [`with_session`] from handing out
/// two `&mut` to it.
struct Slot {
    slot: UnsafeCell<Option<FileServer>>,
    borrowed: Cell<bool>,
}

// SAFETY: this is the claim that there is one thread. It holds today for a reason that
// is measured rather than hoped for: `core::sync::atomic` does not exist on this target
// (`max-atomic-width: 0`, experiment 72 — ARMv5TE has no `LDREX`/`STREX` and euser 9.3
// exports no CAS), so neither `Arc` nor any of Rust's thread-spawning machinery can be
// built, and this SDK exposes no `RThread`. A program that reached for euser's
// `RThread::Create` itself could break the claim; when threads arrive (step 72's
// follow-up) this cell has to become a per-thread session or take the `RFastLock` the
// atomics shim already owns. Until then the `Cell` below is the whole of the
// synchronisation, and it is enough for one thread.
unsafe impl Sync for Slot {}

static SESSION: Slot = Slot {
    slot: UnsafeCell::new(None),
    borrowed: Cell::new(false),
};

/// Runs `f` with the process's file-server session, connecting it on first use.
///
/// `KErrInUse` if it is called again from inside `f`: the session is handed out as
/// `&mut`, so it can only be handed out once at a time. Any other error is the one
/// `RFs::Connect` or `f` itself produced.
pub fn with_session<T>(f: impl FnOnce(&mut FileServer) -> Result<T>) -> Result<T> {
    if SESSION.borrowed.get() {
        return Err(SymbianError::of(ErrorKind::InUse));
    }
    SESSION.borrowed.set(true);
    let result = borrowed(f);
    SESSION.borrowed.set(false);
    result
}

/// The borrow itself, split out so the flag is cleared on every path.
fn borrowed<T>(f: impl FnOnce(&mut FileServer) -> Result<T>) -> Result<T> {
    // SAFETY: `SESSION.borrowed` was just set and is cleared only by the caller after
    // this returns, so this is the only live reference to the slot. There is no second
    // thread (see the `Sync` note above) and `panic = "abort"` means nothing unwinds
    // out of `f` to leave the flag set with a reference still live.
    let slot = unsafe { &mut *SESSION.slot.get() };
    let session = match slot {
        Some(session) => session,
        none => none.insert(FileServer::connect()?),
    };
    f(session)
}

/// Makes `request` on the process's session, connecting it on first use: the one
/// non-generic body every path call of the `std` layer goes through (experiment 105).
///
/// `KErrInUse` inside [`with_session`], as `with_session` itself is: the session is
/// lent out there as `&mut`, and a second user would alias it.
///
/// # Safety
///
/// Every pointer `request` carries is live and valid for what its call writes, as
/// [`Request::on`] requires.
#[inline(never)]
pub(crate) unsafe fn request(path: &str, request: Request<'_>) -> Result<i32> {
    // SAFETY: `with_session` lends the session exclusively for the closure, so nothing
    // else uses it during the call; the pointers are the caller's promise.
    with_session(|fs| unsafe { request.on(fs.as_rfs(), path) })
}

/// The process's session, for the calls that need nothing from it but the path: an
/// application's `fs::remove_file` should not have to name a session.
///
/// Each one is a [`Request`] made by [`request`], so the path is encoded and the session
/// found in one body however many of them an image calls. `KErrInUse` inside
/// [`with_session`].
pub struct ProcessSession;

impl ProcessSession {
    /// Creates every missing directory of `path` (`RFs::MkDirAll`); the last component is
    /// taken as a file name and is not created. An existing path is `KErrAlreadyExists`.
    pub fn make_dir_all(path: &str) -> Result<()> {
        // SAFETY: the request carries no pointer.
        unsafe { request(path, Request::MakeDirAll) }.map(|_| ())
    }

    /// Removes one file (`RFs::Delete`): `KErrInUse` while it is open.
    pub fn delete(path: &str) -> Result<()> {
        // SAFETY: the request carries no pointer.
        unsafe { request(path, Request::Delete) }.map(|_| ())
    }

    /// Renames a file or directory (`RFs::Rename`): `KErrAlreadyExists` if `to` is taken.
    pub fn rename(from: &str, to: &str) -> Result<()> {
        // SAFETY: the request carries a `&str`, not a pointer.
        unsafe { request(from, Request::Rename(to)) }.map(|_| ())
    }
}
