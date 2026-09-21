//! `class RProcess` (`e32std.h` line 3749), from `nm -D
//! epoc32/release/armv5/lib/euser.dso`. Non-static members, called with `this` as
//! argument 0 (the member ABI observed in experiment 78).
//!
//! **None of these leaves**: `e32std.h` declares no leaving member on `RProcess` at
//! all, and every one of them reports failure as a `TInt` or not at all. The one
//! export that is *not* a C signature is `Id()`, which returns an 8-byte `TProcessId`
//! by value and is therefore sret (rule 2 of `symrs_shim.h`); it goes through
//! [`crate::shim::symrs_process_id`].
//!
//! `FileName()` is the other sret one — a `TFileName` by value — and it is
//! [`crate::shim::symrs_process_file_name`].

use crate::des::TDesC16;
use crate::thread::TRequestStatus;

/// `RProcess` is `RHandleBase`: one `TInt iHandle`, and nothing else —
/// `sizeof(RProcess) == 4`, measured by compiling `return sizeof(RProcess);` with the
/// recorded GCCE argv and reading the immediate (`movs r0, #4`).
#[repr(C)]
pub struct RProcess {
    pub handle: i32,
}

/// `EOwnerProcess` of `TOwnerType` (`e32const.h` line 2085): the **first** enumerator,
/// and the default argument of `RProcess::Create`.
pub const EOWNER_PROCESS: i32 = 0;

/// `TExitType` (`e32const.h` line 2241), in declaration order.
pub const EEXIT_KILL: i32 = 0;
/// `EExitTerminate`.
pub const EEXIT_TERMINATE: i32 = 1;
/// `EExitPanic`.
pub const EEXIT_PANIC: i32 = 2;
/// `EExitPending`: the process has not ended yet.
pub const EEXIT_PENDING: i32 = 3;

unsafe extern "C" {
    /// `000014ac T _ZN8RProcess6CreateERK7TDesC16S2_10TOwnerType` —
    /// `RProcess::Create(const TDesC& aFileName, const TDesC& aCommand, TOwnerType)`.
    ///
    /// The new process is created **suspended**; [`RProcess_Resume`] starts it. There
    /// are exactly two things a creator passes: the image and one command-line
    /// descriptor. No environment, no working directory, no file descriptors.
    #[link_name = "_ZN8RProcess6CreateERK7TDesC16S2_10TOwnerType"]
    pub fn RProcess_Create(
        this: *mut RProcess,
        file_name: *const TDesC16,
        command: *const TDesC16,
        owner: i32,
    ) -> i32;

    /// `000014b4 T _ZN8RProcess6ResumeEv` — `RProcess::Resume()`: starts the process
    /// created above. Returns `void`; a failure here is a panic on the creator.
    #[link_name = "_ZN8RProcess6ResumeEv"]
    pub fn RProcess_Resume(this: *mut RProcess);

    /// `00001ce8 T _ZNK8RProcess5LogonER14TRequestStatus` —
    /// `RProcess::Logon(TRequestStatus&) const`: completes the status with the
    /// process's **exit reason** when it ends. If it has already ended, the request
    /// completes at once.
    #[link_name = "_ZNK8RProcess5LogonER14TRequestStatus"]
    pub fn RProcess_Logon(this: *const RProcess, status: *mut TRequestStatus);

    /// `00001cec T _ZNK8RProcess8ExitTypeEv` — `RProcess::ExitType() const`: one of the
    /// `EEXIT_*` constants above, `EExitPending` while it is still running.
    #[link_name = "_ZNK8RProcess8ExitTypeEv"]
    pub fn RProcess_ExitType(this: *const RProcess) -> i32;

    /// `00001cbc T _ZNK8RProcess10ExitReasonEv` — `RProcess::ExitReason() const`: the
    /// value the process ended with — `User::Exit`'s argument, `Kill`'s reason, or a
    /// panic's.
    #[link_name = "_ZNK8RProcess10ExitReasonEv"]
    pub fn RProcess_ExitReason(this: *const RProcess) -> i32;

    /// `0000149c T _ZN8RProcess4KillEi` — `RProcess::Kill(TInt aReason)`: ends the
    /// process as if it had called `User::Exit(aReason)`.
    #[link_name = "_ZN8RProcess4KillEi"]
    pub fn RProcess_Kill(this: *mut RProcess, reason: i32);
}
