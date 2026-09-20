//! A run-time check that the C++ shim's trap harness is really in place.
//!
//! Everything else in this crate is arranged so that a leave can never reach a Rust
//! frame; this is the one place that deliberately raises one, to prove it. It matters
//! because the failure mode has no symptom: with `__LEAVE_EQUALS_THROW__` a leave is a
//! C++ exception, rustc marks the whole Rust text `cantunwind`, and an exception that
//! reaches it ends the process without a panic, a `KERN-EXEC` or a line in the log
//! (experiment 76). A build that silently lost the `TRAP` would look exactly like a
//! build that works, until the first error.
use symbian_sys::shim::symrs_leave_if_error;

use crate::error::{Result, check};

/// Calls `User::LeaveIfError(code)` inside the shim's trap harness.
///
/// A negative `code` really leaves — the same leave experiment 76 used — and comes back
/// as `Err` with that code; zero and positive codes return `Ok`. An application that
/// wants to be sure of its own toolchain can assert that on the device in hand.
pub fn leave_if_error(code: i32) -> Result<()> {
    // SAFETY: the shim function is a complete `TRAP` unit around one euser static
    // member call. It takes and returns a `TInt`, owns nothing, and cannot let a C++
    // exception out, so nothing unwinds into this frame.
    let caught = unsafe { symrs_leave_if_error(code) };
    check(caught).map(|_| ())
}
