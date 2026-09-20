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

use super::server::FileServer;
use crate::error::Result;
use crate::{ErrorKind, SymbianError};

/// The process's session, plus the flag that keeps [`with_session`] from handing out
/// two `&mut` to it.
struct ProcessSession {
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
unsafe impl Sync for ProcessSession {}

static SESSION: ProcessSession = ProcessSession {
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
