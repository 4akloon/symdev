//! `e32err.h` codes as `io::ErrorKind`, and the name `e32err.h` gives each one.
//!
//! This is the mapping `symbian_std::io::ErrorKind::of` already makes (experiment 79),
//! moved to where `std::io::Error::from_raw_os_error` reaches it. The rule is the same:
//! where `e32err.h` and `std` mean the same thing the mapping is exact, and where they
//! do not the code becomes `Uncategorized` rather than being forced into a word that
//! would mislead. The `TInt` is never lost — `io::Error::raw_os_error` still has it.

use crate::fmt;
use crate::io::ErrorKind;

/// There is no `errno` on this platform: every call reports its own `TInt`, either as a
/// return value or through a `TRequestStatus`, and nothing is stored per thread.
pub fn errno() -> i32 {
    0
}

/// Nothing on EKA2 interrupts a user-side wait the way `EINTR` does: a request either
/// completes or is cancelled, and a cancellation is `KErrCancel`, which the caller is
/// meant to see rather than retry.
pub fn is_interrupted(_code: i32) -> bool {
    false
}

pub fn decode_error_kind(code: i32) -> ErrorKind {
    use ErrorKind::*;
    match code {
        -1 | -12 => NotFound,            // KErrNotFound, KErrPathNotFound
        -21 | -46 => PermissionDenied,   // KErrAccessDenied, KErrPermissionDenied
        -11 => AlreadyExists,            // KErrAlreadyExists
        -6 | -8 | -10 => InvalidInput,   // KErrArgument, KErrBadHandle, KErrUnderflow
        -7 | -20 | -38 => InvalidData,   // KErrTotalLossOfPrecision, KErrCorrupt, KErrBadDescriptor
        -28 => InvalidFilename,          // KErrBadName
        -25 => UnexpectedEof,            // KErrEof
        -9 | -40 => FileTooLarge,        // KErrOverflow, KErrTooBig
        -26 | -43 => StorageFull,        // KErrDiskFull, KErrDirFull
        -14 | -22 | -16 => ResourceBusy, // KErrInUse, KErrLocked, KErrServerBusy
        -4 => OutOfMemory,               // KErrNoMemory
        -5 | -47 => Unsupported,         // KErrNotSupported, KErrExtensionNotSupported
        -33 => TimedOut,                 // KErrTimedOut
        -34 => ConnectionRefused,        // KErrCouldNotConnect
        -36 => NotConnected,             // KErrDisconnected
        -15 | -45 => ConnectionAborted,  // KErrServerTerminated, KErrSessionClosed
        -3 => Interrupted,               // KErrCancel: the request was cancelled
        _ => Uncategorized,
    }
}

/// `e32err.h`'s own name for the code, which is what a Symbian developer reads in a
/// panic dialog and in the emulator's log. There is no `strerror` on this platform and
/// no sentence to print, so the name is the message.
pub fn format_error(errno: i32, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match name(errno) {
        Some(name) => f.write_str(name),
        None => write!(f, "Symbian error {errno}"),
    }
}

/// The `KErr*` names of `e32err.h`, in the order the header declares them.
fn name(code: i32) -> Option<&'static str> {
    const NAMES: &[&str] = &[
        "KErrNone",
        "KErrNotFound",
        "KErrGeneral",
        "KErrCancel",
        "KErrNoMemory",
        "KErrNotSupported",
        "KErrArgument",
        "KErrTotalLossOfPrecision",
        "KErrBadHandle",
        "KErrOverflow",
        "KErrUnderflow",
        "KErrAlreadyExists",
        "KErrPathNotFound",
        "KErrDied",
        "KErrInUse",
        "KErrServerTerminated",
        "KErrServerBusy",
        "KErrCompletion",
        "KErrNotReady",
        "KErrUnknown",
        "KErrCorrupt",
        "KErrAccessDenied",
        "KErrLocked",
        "KErrWrite",
        "KErrDisMounted",
        "KErrEof",
        "KErrDiskFull",
        "KErrBadDriver",
        "KErrBadName",
        "KErrCommsLineFail",
        "KErrCommsFrame",
        "KErrCommsOverrun",
        "KErrCommsParity",
        "KErrTimedOut",
        "KErrCouldNotConnect",
        "KErrCouldNotDisconnect",
        "KErrDisconnected",
        "KErrBadLibraryEntryPoint",
        "KErrBadDescriptor",
        "KErrAbort",
        "KErrTooBig",
        "KErrDivideByZero",
        "KErrBadPower",
        "KErrDirFull",
        "KErrHardwareNotAvailable",
        "KErrSessionClosed",
        "KErrPermissionDenied",
        "KErrExtensionNotSupported",
        "KErrCommsBreak",
    ];
    usize::try_from(-code).ok().and_then(|i| NAMES.get(i)).copied()
}
