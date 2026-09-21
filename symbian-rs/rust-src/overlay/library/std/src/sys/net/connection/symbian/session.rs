//! One socket-server session per thread.
//!
//! `es_sock.h` has no `RSocketServ::ShareAuto` and nothing here has observed the socket
//! server accepting a shared session, so a session is not passed between threads. Every
//! thread that opens a socket gets its own `RSocketServ::Connect`, closed when the
//! thread ends through the ordinary thread-local destructor — the same arrangement, for
//! the same reason, as `sys::fs::symbian`'s file-server session.
//!
//! The consequence is the same divergence, and it is written down rather than hidden:
//! **a socket should be used, and dropped, on the thread that opened it.** `std`'s
//! `TcpStream` is `Send` and nothing here can stop it crossing; the sub-session inside
//! it belongs to the session that opened it, and the socket server answers
//! `KErrBadHandle` to a thread that does not own it.

use crate::cell::RefCell;
use crate::io;
use symbian_sys::esock::{
    KESOCK_DEFAULT_MESSAGE_SLOTS, RSocketServ, RSocketServ_Close, RSocketServ_Connect,
};

/// A connected session, closed on drop.
pub struct Session {
    server: RSocketServ,
}

impl Session {
    fn connect() -> io::Result<Self> {
        let mut session = Session { server: RSocketServ { handle: 0 } };
        // SAFETY: `RSocketServ` is the observed one-word layout (`sizeof == 4`) passed
        // as `this`, argument 0, per the member ABI observed in experiment 78.
        // `Connect` is non-leaving and reports every failure as the returned `TInt`; on
        // failure the handle stays zero, which `Drop` is safe on.
        let code = unsafe { RSocketServ_Connect(&mut session.server, KESOCK_DEFAULT_MESSAGE_SLOTS) };
        if code != 0 {
            return Err(capability_hint(code));
        }
        Ok(session)
    }

    /// The `RSocketServ&` an `RSocket::Open` or `RHostResolver::Open` takes.
    pub fn as_server(&mut self) -> *mut RSocketServ {
        &raw mut self.server
    }
}

/// `KErrPermissionDenied` from the socket server means the process was not granted
/// **`NetworkServices`** — the server checks its connection policy before any socket
/// exists, so this is the first and clearest place the capability shows up, and a bare
/// "permission denied" would send the reader looking at the file system.
fn capability_hint(code: i32) -> io::Error {
    if code == -46 {
        io::Error::new(
            io::ErrorKind::PermissionDenied,
            "the socket server refused the session: this process has no NetworkServices \
             capability (symdev.toml: capabilities = [\"NetworkServices\"])",
        )
    } else {
        io::Error::from_raw_os_error(code)
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        // SAFETY: `RSocketServ::Close` is a non-leaving member taking only `this`, and
        // it is safe on a handle that was never opened (the word is zero then).
        unsafe { RSocketServ_Close(&mut self.server) };
    }
}

thread_local! {
    static SESSION: RefCell<Option<Session>> = const { RefCell::new(None) };
}

/// Runs `f` with this thread's session, connecting on first use.
///
/// A re-entrant call is refused with `ResourceBusy` rather than allowed to alias the
/// session, which is what `RefCell` would otherwise panic about inside a platform layer
/// that must not unwind. It matters more here than for files: accepting a connection
/// needs the session *inside* the accept.
pub fn with_session<T>(f: impl FnOnce(&mut Session) -> io::Result<T>) -> io::Result<T> {
    SESSION.with(|cell| {
        let Ok(mut slot) = cell.try_borrow_mut() else {
            return Err(io::Error::new(
                io::ErrorKind::ResourceBusy,
                "the socket-server session of this thread is already in use",
            ));
        };
        let session = match &mut *slot {
            Some(session) => session,
            none => none.insert(Session::connect()?),
        };
        f(session)
    })
}
