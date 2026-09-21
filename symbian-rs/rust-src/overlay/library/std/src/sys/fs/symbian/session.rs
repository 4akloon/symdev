//! One file-server session per thread.
//!
//! A Symbian kernel handle belongs to the thread that made it, and an `RFs` session is
//! not shareable between threads until somebody calls `RFs::ShareProtected` —
//! behaviour this SDK has never observed, so it is not relied on. Every thread that
//! touches a file therefore gets its own `RFs::Connect`, closed when the thread ends
//! through the ordinary thread-local destructor.
//!
//! The consequence is a real divergence from `std` and is written down rather than
//! hidden: **an open `File` should be used on the thread that opened it.** `std`'s
//! `File` is `Send`, and nothing here can stop it crossing, but the `RFile` sub-session
//! inside it belongs to the session that opened it and the file server answers
//! `KErrBadHandle` to a thread that does not own it.

use crate::cell::RefCell;
use crate::io;
use symbian_sys::efsrv::{KFILE_SERVER_DEFAULT_MESSAGE_SLOTS, RFs, RFs_Connect};
use symbian_sys::euser::{RHandleBase, RHandleBase_Close};

/// An open session, closed on drop.
pub struct Session {
    fs: RFs,
}

impl Session {
    fn connect() -> io::Result<Self> {
        let mut session = Session { fs: RFs { handle: 0 } };
        // SAFETY: `RFs` is the observed one-word layout (`sizeof(RFs) == 4`) passed as
        // `this`, argument 0, per the member ABI observed in experiment 78. `Connect`
        // is non-leaving and reports every failure as the returned `TInt`.
        let code = unsafe { RFs_Connect(&mut session.fs, KFILE_SERVER_DEFAULT_MESSAGE_SLOTS) };
        if code != 0 {
            return Err(io::Error::from_raw_os_error(code));
        }
        Ok(session)
    }

    pub fn as_rfs(&mut self) -> *mut RFs {
        &mut self.fs
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        // SAFETY: `RFs::Close()` compiles to a bare `RHandleBase::Close()` (observed),
        // is non-leaving, and is safe on a handle that was never opened.
        unsafe { RHandleBase_Close((&raw mut self.fs).cast::<RHandleBase>()) };
    }
}

thread_local! {
    static SESSION: RefCell<Option<Session>> = const { RefCell::new(None) };
}

/// Runs `f` with this thread's session, connecting on first use.
///
/// A re-entrant call — a closure that reaches the file system again — is refused with
/// `ResourceBusy` rather than allowed to alias the session, which is what `RefCell`
/// would otherwise panic about inside a platform layer that must not unwind.
pub fn with_session<T>(f: impl FnOnce(&mut Session) -> io::Result<T>) -> io::Result<T> {
    SESSION.with(|cell| {
        let Ok(mut slot) = cell.try_borrow_mut() else {
            return Err(io::Error::new(
                io::ErrorKind::ResourceBusy,
                "the file-server session of this thread is already in use",
            ));
        };
        let session = match &mut *slot {
            Some(session) => session,
            none => none.insert(Session::connect()?),
        };
        f(session)
    })
}
