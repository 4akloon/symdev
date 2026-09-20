//! Safe Symbian OS 9.3 domain types: what an application uses so it writes no `unsafe`
//! and never sees a raw C function (design spec §7, experiment 69).
//!
//! - [`SymbianError`] keeps the raw `TInt` a call returned and names it from `e32err.h`.
//! - [`des`] is the 16-bit descriptor family, with every layout observed rather than
//!   assumed; the observation table is in that module's documentation.
//! - [`user`] wraps the non-leaving `User::` exports. Anything that can leave waits for
//!   the C++ `TRAP` shim of step 70 and is deliberately absent.
#![no_std]

extern crate alloc;

pub mod des;
pub mod user;

mod error;
mod error_kind;

pub use des::{Buf16, Des16, DesC16, HBuf16, PtrC16};
pub use error::{Result, SymbianError, check};
pub use error_kind::ErrorKind;

/// The layouts the descriptor types must keep, checked at compile time against what the
/// C++ probe measured on the device (see the `des` module documentation).
const _: () = {
    assert!(size_of::<Buf16<8>>() == 24);
    assert!(size_of::<Buf16<16>>() == 40);
    assert!(align_of::<Buf16<8>>() == 4);
    assert!(align_of::<PtrC16<'static>>() == 4);
};
