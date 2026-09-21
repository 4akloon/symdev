//! The child itself: `RProcess`, and how it ended.
//!
//! Symbian says how a process ended in **two** values, not one: `ExitType()` — killed,
//! terminated or panicked — and `ExitReason()`. `std`'s `ExitStatus` carries both, and
//! `code()` answers `Some` only for `EExitKill`, which is the ordinary end (including
//! `User::Exit(n)` and a `main` that returned `n`). A panic has a category and a
//! reason and is not an exit code, so `code()` is `None` there, exactly as it is for a
//! signal on Unix.

use crate::ffi::{OsStr, OsString};
use crate::num::NonZero;
use crate::sys::pal::symbian::des::PathBuf16;
use crate::sys::pal::symbian::request::blocking;
use crate::{fmt, io};
use symbian_sys::process::{
    EEXIT_KILL, EEXIT_PANIC, EEXIT_PENDING, EEXIT_TERMINATE, EOWNER_PROCESS, RProcess,
    RProcess_Create, RProcess_ExitReason, RProcess_ExitType, RProcess_Kill, RProcess_Logon,
    RProcess_Resume,
};

/// A spawned process, closed on drop.
pub struct Process {
    handle: RProcess,
}

impl Process {
    /// `RProcess::Create` with the arguments joined into one command line, then
    /// `Resume`.
    pub(super) fn spawn(program: &OsStr, args: &[OsString]) -> io::Result<Self> {
        let program = to_utf8(program)?;
        let mut command = crate::string::String::new();
        for arg in args {
            if !command.is_empty() {
                command.push(' ');
            }
            command.push_str(to_utf8(arg)?);
        }
        let image = PathBuf16::new(program)?;
        let command = PathBuf16::new(&command)?;
        let mut process = Process { handle: RProcess { handle: 0 } };
        // SAFETY: `this` in argument 0 per the observed member ABI, then two borrowed
        // descriptors the loader only reads and a scalar. Non-leaving, and every
        // failure is the returned `TInt`; on failure the handle stays zero, which
        // `Drop` is safe on.
        let code = unsafe {
            RProcess_Create(
                &mut process.handle,
                image.as_tdesc16(),
                command.as_tdesc16(),
                EOWNER_PROCESS,
            )
        };
        if code != 0 {
            return Err(io::Error::from_raw_os_error(code));
        }
        // SAFETY: the process was just created and is suspended; `Resume` is a
        // non-leaving member taking only `this` and returning `void`.
        unsafe { RProcess_Resume(&mut process.handle) };
        Ok(process)
    }

    pub fn id(&self) -> u32 {
        // SAFETY: a live `RProcess` of the measured one-word layout; the shim only
        // reads its id. See `sys::process::symbian::getpid` for why it is a shim.
        unsafe { symbian_sys::shim::symrs_process_id(&self.handle) }
    }

    /// `RProcess::Kill`, which ends the child as if it had called `User::Exit`.
    pub fn kill(&mut self) -> io::Result<()> {
        // SAFETY: a non-leaving member taking `this` and a scalar, returning `void`.
        // Killing an already-dead process is a no-op.
        unsafe { RProcess_Kill(&mut self.handle, 0) };
        Ok(())
    }

    /// `RProcess::Logon`, then `User::WaitForRequest`.
    ///
    /// Logon completes the status with the child's exit reason; if the child has
    /// already ended, it completes at once. Like every blocking wait in this `std`,
    /// it refuses under a `CActiveScheduler` rather than eating its completions — see
    /// `sys::pal::symbian::request`.
    pub fn wait(&mut self) -> io::Result<ExitStatus> {
        let handle = &raw const self.handle;
        blocking(|status| {
            // SAFETY: `this` in argument 0 on a live handle, and the status is the
            // live one `blocking` waits for immediately afterwards. Non-leaving.
            unsafe { RProcess_Logon(handle, status) };
        })?;
        Ok(self.status())
    }

    pub fn try_wait(&mut self) -> io::Result<Option<ExitStatus>> {
        let status = self.status();
        Ok((status.exit_type != EEXIT_PENDING).then_some(status))
    }

    fn status(&self) -> ExitStatus {
        // SAFETY: two non-leaving `const` members taking only `this`, on a live
        // handle; they read the kernel's record of how the process ended.
        unsafe {
            ExitStatus {
                exit_type: RProcess_ExitType(&self.handle),
                reason: RProcess_ExitReason(&self.handle),
            }
        }
    }
}

impl Drop for Process {
    fn drop(&mut self) {
        // SAFETY: `RHandleBase::Close` is a non-leaving member taking only `this`, and
        // it is safe on a handle that was never opened (the word is zero then).
        // Closing the handle does not end the process: a Symbian child outlives its
        // creator, which is what `std::process::Child`'s drop means too.
        unsafe {
            symbian_sys::euser::RHandleBase_Close(
                (&raw mut self.handle).cast::<symbian_sys::euser::RHandleBase>(),
            )
        };
    }
}

fn to_utf8(s: &OsStr) -> io::Result<&str> {
    s.to_str().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "a Symbian command line is UTF-16 and is built from a UTF-8 Rust string",
        )
    })
}

/// How a process ended: the `TExitType` and the `TInt` reason.
#[derive(PartialEq, Eq, Clone, Copy, Debug, Default)]
pub struct ExitStatus {
    exit_type: i32,
    reason: i32,
}

impl ExitStatus {
    pub fn exit_ok(&self) -> Result<(), ExitStatusError> {
        if self.exit_type == EEXIT_KILL && self.reason == 0 {
            Ok(())
        } else {
            Err(ExitStatusError(*self))
        }
    }

    /// `Some` only for the ordinary end. A panic is a category and a reason, not an
    /// exit code, so it answers `None` — as a signal does on Unix.
    pub fn code(&self) -> Option<i32> {
        (self.exit_type == EEXIT_KILL).then_some(self.reason)
    }
}

impl fmt::Display for ExitStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.exit_type {
            EEXIT_KILL => write!(f, "exit code: {}", self.reason),
            EEXIT_TERMINATE => write!(f, "terminated with reason {}", self.reason),
            EEXIT_PANIC => write!(f, "panicked with reason {}", self.reason),
            EEXIT_PENDING => write!(f, "still running"),
            other => write!(f, "TExitType {other} with reason {}", self.reason),
        }
    }
}

/// A non-zero end, in the shape `ExitStatus::exit_ok` promises.
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub struct ExitStatusError(ExitStatus);

impl From<ExitStatusError> for ExitStatus {
    fn from(error: ExitStatusError) -> ExitStatus {
        error.0
    }
}

impl ExitStatusError {
    pub fn code(self) -> Option<NonZero<i32>> {
        NonZero::new(self.0.reason)
    }
}

/// What *this* process ends with, which the entry point returns to `E32Main`.
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub struct ExitCode(u8);

impl ExitCode {
    pub const SUCCESS: ExitCode = ExitCode(0);
    pub const FAILURE: ExitCode = ExitCode(1);

    pub fn as_i32(&self) -> i32 {
        self.0 as i32
    }
}

impl From<u8> for ExitCode {
    fn from(code: u8) -> Self {
        Self(code)
    }
}
