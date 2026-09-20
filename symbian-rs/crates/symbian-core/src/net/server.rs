//! `SocketServer` (`RSocketServ`) and the one session a process has.
//!
//! Symbian's convention is one `RSocketServ` per thread, connected once and kept for the
//! life of the program, with every socket and resolver opened as a sub-session of it.
//! Closing the session closes all of them, so [`with_session`] connects on first use and
//! **never** closes: the kernel closes every handle when the process ends, and dropping
//! the session while a socket opened from it is still alive would be the bug, not the
//! leak. It is the same arrangement, for the same reason, as `crate::fs`'s file-server
//! session.
use core::cell::{Cell, UnsafeCell};

use symbian_sys::esock::{
    KESOCK_DEFAULT_MESSAGE_SLOTS, RSocketServ, RSocketServ_Close, RSocketServ_Connect,
};

use crate::error::{Result, check};
use crate::{ErrorKind, SymbianError};

/// A connected session with the socket server.
///
/// Not `Send` or `Sync`: a session handle belongs to the thread that opened it.
pub struct SocketServer {
    server: RSocketServ,
}

impl SocketServer {
    /// Connects to the socket server (`RSocketServ::Connect`).
    ///
    /// `KErrPermissionDenied` (-46) here means the process was not granted
    /// **`NetworkServices`**: the server checks its connection policy before any socket
    /// exists, so this is the first and clearest place the capability shows up.
    /// `symbian_std::net` turns that code into a message naming the capability and the
    /// manifest key.
    pub fn connect() -> Result<Self> {
        let mut server = RSocketServ { handle: 0 };
        // SAFETY: `this` in argument 0 per the observed member ABI, then a scalar. The
        // call is non-leaving (`es_sock.h` lines 671-717 declare no leaving member) and
        // reports every failure as the returned `TInt`; on failure the handle is left
        // as it was, and `Drop` is safe on a zero handle.
        let code = unsafe { RSocketServ_Connect(&mut server, KESOCK_DEFAULT_MESSAGE_SLOTS) };
        check(code)?;
        Ok(Self { server })
    }

    /// The `RSocketServ&` an `RSocket::Open` or `RHostResolver::Open` takes.
    pub(crate) const fn as_server(&mut self) -> *mut RSocketServ {
        &raw mut self.server
    }
}

impl Drop for SocketServer {
    fn drop(&mut self) {
        // SAFETY: `RHandleBase::Close` is a non-leaving member taking only `this`, and
        // it is safe on a handle that was never opened (the word is zero then).
        unsafe { RSocketServ_Close(&mut self.server) }
    }
}

/// The process's session, plus the flag that keeps [`with_session`] from handing out two
/// `&mut` to it.
struct ProcessSession {
    slot: UnsafeCell<Option<SocketServer>>,
    borrowed: Cell<bool>,
}

// SAFETY: this is the claim that there is one thread, and it is the same claim
// `crate::fs`'s session makes, for the same measured reason: `core::sync::atomic` does
// not exist on this target (`max-atomic-width: 0`, experiment 72), so nothing in this
// SDK can spawn a thread. When threads arrive this cell has to become per-thread or take
// the `RFastLock` the atomics shim owns. Until then the `Cell` below is the whole of the
// synchronisation, and it is enough for one thread.
unsafe impl Sync for ProcessSession {}

static SESSION: ProcessSession = ProcessSession {
    slot: UnsafeCell::new(None),
    borrowed: Cell::new(false),
};

/// Runs `f` with the process's socket-server session, connecting it on first use.
///
/// `KErrInUse` if it is called again from inside `f`: the session is handed out as
/// `&mut`, so it can only be handed out once at a time. That matters more here than for
/// files, because accepting a connection needs the session inside the accept.
pub fn with_session<T>(f: impl FnOnce(&mut SocketServer) -> Result<T>) -> Result<T> {
    if SESSION.borrowed.get() {
        return Err(SymbianError::of(ErrorKind::InUse));
    }
    SESSION.borrowed.set(true);
    let result = borrowed(f);
    SESSION.borrowed.set(false);
    result
}

/// The borrow itself, split out so the flag is cleared on every path.
fn borrowed<T>(f: impl FnOnce(&mut SocketServer) -> Result<T>) -> Result<T> {
    // SAFETY: `SESSION.borrowed` was just set and is cleared only by the caller after
    // this returns, so this is the only live reference to the slot. There is no second
    // thread (see the `Sync` note above) and `panic = "abort"` means nothing unwinds out
    // of `f` to leave the flag set with a reference still live.
    let slot = unsafe { &mut *SESSION.slot.get() };
    let session = match slot {
        Some(session) => session,
        none => none.insert(SocketServer::connect()?),
    };
    f(session)
}
