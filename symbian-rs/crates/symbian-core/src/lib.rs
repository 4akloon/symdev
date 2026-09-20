//! Safe Symbian OS 9.3 domain types: what an application uses so it writes no `unsafe`
//! and never sees a raw C function (design spec §7, experiment 69).
//!
//! - [`SymbianError`] keeps the raw `TInt` a call returned and names it from `e32err.h`.
//! - [`des`] is the 16-bit descriptor family, with every layout observed rather than
//!   assumed; the observation table is in that module's documentation.
//! - [`des8`] is as much of the 8-bit descriptor family as the file API needs, built
//!   by euser's own constructors because its type nibbles were never observed.
//! - [`fs`] is the file system — the session, an open file and a directory entry — and
//!   the place where both halves of the shim rule appear: every `RFs` and `RFile`
//!   member is called directly because none of them can leave, while
//!   `BaflUtils::EnsurePathExistsL` goes through the C++ `TRAP` shim because it can.
//! - [`net`] is sockets — the session, a socket, the resolver and an address. Both
//!   halves of the shim rule land on the same side there: nothing in `es_sock.h` or
//!   `in_sock.h` leaves, so the whole subsystem is called directly.
//! - [`shim`] is the run-time check that the C++ trap harness is in place.
//! - [`time`] is the clocks: the three `User` counters and the `TTime` wall clock.
//! - [`user`] wraps the non-leaving `User::` exports.
//!
//! What needs a shim and what does not is written out once, in
//! `symbian-rs/shims/common/symrs_shim.h`.
#![no_std]

extern crate alloc;

pub mod des;
pub mod des8;
pub mod fs;
pub mod net;
pub mod shim;
pub mod time;
pub mod user;

mod error;
mod error_kind;

pub use des::{Buf16, Des16, DesC16, HBuf16, PtrC16};
pub use des8::{DesC8, Ptr8, PtrC8};
pub use error::{Result, SymbianError, check};
pub use error_kind::ErrorKind;
pub use fs::{Entry, File, FileServer};

/// The layouts the descriptor types must keep, checked at compile time against what the
/// C++ probe measured on the device (see the `des` module documentation).
const _: () = {
    assert!(size_of::<Buf16<8>>() == 24);
    assert!(size_of::<Buf16<16>>() == 40);
    assert!(align_of::<Buf16<8>>() == 4);
    assert!(align_of::<PtrC16<'static>>() == 4);
};
