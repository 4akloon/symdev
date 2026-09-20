//! The C++ shim's `extern "C"` entry points (`symbian-rs/shims/`, design spec §7,
//! step 70).
//!
//! Everything here is defined in this repository, not in a Symbian DLL: symdev compiles
//! `shims/common/*.cpp` with the recorded C++ argv and archives the objects next to the
//! Rust archive on the link line. The application names none of it.
//!
//! A shim function is a complete `TRAP` unit and returns a `TInt`: `KErrNone`, the leave
//! code, or `KErrArgument` for a null pointer. It exists because a leave is a real C++
//! exception here (`__LEAVE_EQUALS_THROW__`) and rustc marks the whole Rust text
//! `cantunwind`, so an exception that reaches a Rust frame kills the process with no
//! diagnostic at all (experiment 76). The rule for what needs a wrapper is in
//! `shims/common/symrs_shim.h`; read it before adding one.

use crate::des::TDesC16;
use crate::efsrv::RFs;

unsafe extern "C" {
    /// `BaflUtils::EnsurePathExistsL(RFs&, const TDesC&)`
    /// (`_ZN9BaflUtils17EnsurePathExistsLER3RFsRK7TDesC16`, bafl.dso), TRAPped.
    ///
    /// Creates every missing directory in `path`'s path component. It leaves with the
    /// file server's own error — a drive that is not there, a read-only path — and the
    /// wrapper hands that code back instead of letting the exception fly.
    pub fn symrs_bafl_ensure_path_exists(fs: *mut RFs, path: *const TDesC16) -> i32;
}

unsafe extern "C" {
    /// `User::LeaveIfError(TInt)` (`_ZN4User12LeaveIfErrorEi`, euser.dso), TRAPped.
    ///
    /// The shim's self-check: euser's own error-to-leave converter, called inside the
    /// trap harness, so a negative `reason` comes back as itself instead of ending the
    /// process. It is the same leave experiment 76 used to prove the mechanism, and the
    /// only call in the SDK that leaves on demand — which is what makes it a run-time
    /// check that the harness is really there on the machine in hand.
    pub fn symrs_leave_if_error(reason: i32) -> i32;
}
