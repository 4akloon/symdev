//! `FileServer`: a session with the Symbian file server (`RFs`), and the first place
//! where the two halves of the shim rule stand side by side (design spec §7, step 70).
//!
//! - [`FileServer::make_dir_all`] is `RFs::MkDirAll`, a **non-leaving** non-static
//!   member function. It is called straight from Rust with no C++ in the path, because
//!   `f32file.h` declares no leaving member on `RFs` and the member ABI was observed
//!   (experiment 78).
//! - [`FileServer::ensure_path_exists`] is `BaflUtils::EnsurePathExistsL`, which
//!   **leaves**. It goes through the C++ shim, which `TRAP`s it and returns the code.
//!   Calling it directly would end the process without a word the first time a path was
//!   wrong (experiment 76).
//!
//! Both report failure the same way here: a `SymbianError` holding the exact `TInt`.
//! Both also take a `&str`, not a descriptor: an application using this SDK should not
//! have to name a Symbian type to open a path.
use symbian_sys::efsrv::{KFILE_SERVER_DEFAULT_MESSAGE_SLOTS, RFs, RFs_Connect, RFs_MkDirAll};
use symbian_sys::euser::{RHandleBase, RHandleBase_Close};
use symbian_sys::shim::symrs_bafl_ensure_path_exists;

use crate::des::{Buf16, DesC16};
use crate::error::{Result, check};

/// `KMaxFileName` (`e32const.h` line 390: `const TInt KMaxFileName=0x100;`): the longest
/// path the file server accepts, and the size of the descriptor a path is built into.
pub const MAX_FILE_NAME: usize = 0x100;

/// A path is a `&str` in the application and a descriptor by the time it reaches the
/// file server. `KErrOverflow` if it is longer than `KMaxFileName`, `KErrArgument` if it
/// is not valid UTF-16 — the same codes every other conversion in this crate reports.
///
/// Drive letters are Symbian's, not POSIX's: `C:\`, `E:\`, `Z:\`.
fn path_of(path: &str) -> Result<Buf16<MAX_FILE_NAME>> {
    let mut buf = Buf16::new();
    buf.push_str(path)?;
    Ok(buf)
}

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

    /// Creates every directory of `path` that does not exist yet (`RFs::MkDirAll`).
    ///
    /// `path` is a full file name: the last component is taken as the file and is not
    /// created. An existing path is `KErrAlreadyExists`, not success.
    ///
    /// **No shim.** The call cannot leave, so Rust makes it itself.
    pub fn make_dir_all(&mut self, path: &str) -> Result<()> {
        let path = path_of(path)?;
        // SAFETY: `this` in argument 0, the descriptor borrowed for the call and only
        // read. Non-leaving, so no C++ exception can cross this frame.
        let code = unsafe { RFs_MkDirAll(&mut self.fs, path.as_tdesc16()) };
        check(code).map(|_| ())
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
}

impl Drop for FileServer {
    fn drop(&mut self) {
        // SAFETY: `RFs` and `RHandleBase` are the same one-word layout, and `RFs::Close`
        // compiles to exactly this call (observed). It is non-leaving and safe on a
        // handle that is already 0.
        unsafe { RHandleBase_Close((&mut self.fs as *mut RFs).cast::<RHandleBase>()) }
    }
}
