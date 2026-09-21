//! The Symbian OS 9.3 (EKA2, S60 3rd FP2) platform layer.
//!
//! What is here is the small part of the PAL that is not one of the per-facility
//! backends in `sys/*/symbian.rs`: process start-up, process end, and the shape of an
//! "unsupported" answer. Everything else lives beside the other platforms'
//! implementations of the same facility.
//!
//! The calls all come from `symbian_sys`, the SDK's raw declarations, whose mangled
//! names were read off this ROM's import libraries with `nm -D`. The crate is copied
//! into `library/` when the patched source tree is materialised; see
//! `symbian-rs/rust-src/README.md`.

#![deny(unsafe_op_in_unsafe_fn)]

use crate::io as std_io;

/// `KErrGeneral` (`e32err.h`), the reason a Rust panic ends the process with.
///
/// A panic is not a leave and not a clean exit, so it gets a category of its own:
/// `User::Panic(_L("RUST"), KErrGeneral)` is what the loader and the emulator log
/// report, the same shape a C++ panic on this platform has.
const KERR_GENERAL: i32 = -2;

/// The panic category. `Lit16` is the `_LIT16` layout observed in experiment 65a.
static RUST_CATEGORY: symbian_sys::des::Lit16<4> = symbian_sys::des::Lit16::ascii(b"RUST");

// SAFETY: must be called only once during runtime initialization.
// NOTE: this is not guaranteed to run, for example when Rust code is called externally.
pub unsafe fn init(_argc: isize, _argv: *const *const u8, _sigpipe: u8) {}

// SAFETY: must be called only once during runtime cleanup.
// NOTE: this is not guaranteed to run, for example when the program aborts.
pub unsafe fn cleanup() {}

pub fn unsupported<T>() -> std_io::Result<T> {
    Err(unsupported_err())
}

pub fn unsupported_err() -> std_io::Error {
    std_io::Error::UNSUPPORTED_PLATFORM
}

/// Ends the process now, without unwinding and without running anything further.
///
/// `core::intrinsics::abort()`, which the `unsupported` PAL uses, compiles to an
/// undefined instruction; on EKA2 that surfaces as a `KERN-EXEC 3` from the exception
/// handler, which says nothing about where it came from. `User::Panic` is the
/// platform's own way for a program to die on purpose, and it carries a category and a
/// reason that the emulator's log and a device's error dialog both show.
pub fn abort_internal() -> ! {
    // SAFETY: `User::Panic` is a euser static member function (plain EABI, no `this`)
    // that never returns. The category is a `'static` literal descriptor of the layout
    // euser reads a `const TDesC16&` as, and euser copies what it needs before the
    // process goes away.
    unsafe { symbian_sys::euser::User_Panic(RUST_CATEGORY.as_desc(), KERR_GENERAL) }
}
