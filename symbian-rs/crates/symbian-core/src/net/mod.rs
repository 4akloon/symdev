//! Sockets: the safe Symbian types the `std`-shaped `symbian_std::net` is built on.
//!
//! This is the layer that owns the `unsafe`, as [`crate::fs`] does for files, and the
//! same two halves of the step-70 rule turn up — except that here **both** halves come
//! out the same way: `es_sock.h` declares no leaving member on `RSocketServ`, `RSocket`
//! or `RHostResolver`, and `in_sock.h` declares none at all, so every call is made
//! directly with `this` as argument 0 and no C++ shim exists for this subsystem.
//!
//! # The asynchronous shape, and why there is no executor
//!
//! `RSocket::Connect`, `Send`, `RecvOneOrMore`, `Accept` and `Shutdown` take a
//! `TRequestStatus&` and complete later. The blocking form of that is the request
//! followed by `User::WaitForRequest`, which is what [`request::blocking`] does and what
//! `std::net` means. No `CActive`, no `CActiveScheduler`, no Rust executor: step 73's
//! async layer will sit beside this on the same `symbian-sys` declarations rather than
//! underneath it.
//!
//! # What this layer does not decide
//!
//! **The access point.** `RSocket::Open` and `RHostResolver::Open` both have an overload
//! taking an `RConnection`, which is how a Symbian program chooses the IAP a phone
//! dials. This crate uses the overload without one, so the socket server picks — which
//! on a device means the default connection or a dialog, and on EKA2L1 means the host's
//! own networking. There is no `std` concept for the choice and this SDK does not invent
//! one; see the `symbian_std::net` crate documentation.
mod addr;
mod request;
mod resolver;
mod server;
mod socket;

pub use addr::InetAddr;
pub use request::blocking;
pub use resolver::{HostResolver, MAX_HOST_NAME};
pub use server::{SocketServer, with_session};
pub use socket::{Shutdown, Socket};
