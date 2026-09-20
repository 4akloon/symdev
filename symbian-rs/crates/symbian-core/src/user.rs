//! Safe wrappers around the non-leaving `User::` exports (design spec §7).
//!
//! Only calls that cannot leave belong here. A leaving call needs the C++ `TRAP` shim of
//! step 70 and is absent rather than guessed.
use symbian_sys::euser::{User_After, User_Exit, User_InfoPrint};

use crate::des::DesC16;
use crate::error::{Result, check};

/// Shows `text` in the system notifier (`User::InfoPrint`).
///
/// Non-leaving; it returns a system-wide error code, which becomes the `Err`.
pub fn info_print(text: &impl DesC16) -> Result<()> {
    // SAFETY: `text` is one of the observed descriptor layouts, borrowed for the whole
    // call, and `User::InfoPrint` is a euser static member function (plain EABI, no
    // `this`) that only reads it and does not take ownership. It cannot leave, so no
    // C++ exception can cross this frame.
    let code = unsafe { User_InfoPrint(text.as_tdesc16()) };
    check(code).map(|_| ())
}

/// Blocks the calling thread for `micros` microseconds (`User::After`).
pub fn after(micros: i32) {
    // SAFETY: `TTimeIntervalMicroSeconds32` is a 4-byte class passed in one register
    // under the EABI (experiment 65a); the call owns nothing, blocks this thread only
    // and cannot leave.
    unsafe { User_After(micros) }
}

/// Ends the process with `reason` (`User::Exit`). Nothing unwinds.
pub fn exit(reason: i32) -> ! {
    // SAFETY: a euser static member function that never returns and takes the process
    // down without unwinding, as `panic = "abort"` requires.
    unsafe { User_Exit(reason) }
}
