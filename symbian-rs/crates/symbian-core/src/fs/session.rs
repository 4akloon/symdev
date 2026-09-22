//! The one file-server session a process has, so that a caller can write
//! `fs::File::open(path)` without first naming an `RFs`.
//!
//! Symbian's own convention is one `RFs` per thread, connected once and kept for the
//! life of the program: a session is an IPC connection to a server, not a cheap value.
//! The session is connected on first use. The calls this crate makes on it — opening a
//! file, deleting one, asking for an entry — go through [`request`], one non-generic
//! body that finds the session and encodes the path (experiment 105), and
//! [`ProcessSession`] names the ones that need nothing but a path. [`with_session`]
//! lends the session itself to a caller's closure. An `RFile` needs no session
//! afterwards: it carries its own sub-session handle.
//!
//! The session is deliberately never closed: it lives until the process ends, and the
//! kernel closes every handle then. Dropping it while an `RFile` opened from it is
//! still alive would be the bug, not the leak.
use core::cell::{Cell, UnsafeCell};

use symbian_sys::efsrv::{RFs_Delete, RFs_MkDirAll};

use super::request::{Request, rename};
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
    f(connected(slot)?)
}

/// The session in `slot`, connected first if it is not yet.
///
fn connected(slot: &mut Option<FileServer>) -> Result<&mut FileServer> {
    if slot.is_none() {
        let session = FileServer::connect()?;
        // SAFETY: `slot` is a live `&mut` and holds `None`, which owns nothing, so
        // writing over it without a drop loses nothing. (`*slot = …` would drop it,
        // and the compiler does not see that it is still `None` after the call.)
        unsafe { core::ptr::from_mut(slot).write(Some(session)) };
    }
    slot.as_mut().ok_or(SymbianError::of(ErrorKind::General))
}

/// Makes `request` on the process's session, connecting it on first use: the one
/// non-generic body every path call of the `std` layer goes through (experiment 105).
///
/// `KErrInUse` inside [`with_session`], as `with_session` itself is: the session is
/// lent out there as `&mut`, and a second user would alias it.
#[inline(never)]
pub(crate) fn request(path: &str, request: Request<'_>) -> Result<i32> {
    if SESSION.borrowed.get() {
        return Err(SymbianError::of(ErrorKind::InUse));
    }
    // SAFETY: the flag is clear, so `with_session` has lent no reference to the slot,
    // and this one ends before the function does: `Request::on` runs only this
    // module's own calls, none of which comes back here or to `with_session`. One
    // thread (the `Sync` note above).
    let session = connected(unsafe { &mut *SESSION.slot.get() })?;
    // SAFETY: the session is connected and nothing else uses it for the call.
    unsafe { request.on(session.as_rfs(), path) }
}

/// The process's session, for the calls that need nothing from it but the path: an
/// application's `fs::remove_file` should not have to name a session.
///
/// Each one is a [`Request`] made by [`request`], so the path is encoded and the session
/// found in one body however many of them an image calls. `KErrInUse` inside
/// [`with_session`].
pub struct ProcessSession;

impl ProcessSession {
    /// Creates the directory `path` names and every missing parent (`RFs::MkDirAll`,
    /// with the trailing backslash it needs to create the last component too, added
    /// when `path` has none). An existing directory is `KErrAlreadyExists`.
    pub fn make_dirs(path: &str) -> Result<()> {
        // SAFETY, every call here: `this` first per the observed member ABI, the paths
        // borrowed and only read. Non-leaving, every failure is the returned `TInt`.
        let call = &mut |fs, path| unsafe { RFs_MkDirAll(fs, path) };
        request(path, Request::new(call).of_directory(path)).map(|_| ())
    }

    /// Removes one file (`RFs::Delete`): `KErrInUse` while it is open.
    pub fn delete(path: &str) -> Result<()> {
        request(
            path,
            Request::new(&mut |fs, path| unsafe { RFs_Delete(fs, path) }),
        )
        .map(|_| ())
    }

    /// Renames a file or directory (`RFs::Rename`): `KErrAlreadyExists` if `to` is taken.
    pub fn rename(from: &str, to: &str) -> Result<()> {
        let call = &mut |fs, from| unsafe { rename(fs, from, to) };
        request(from, Request::new(call)).map(|_| ())
    }
}
