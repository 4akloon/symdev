//! What the compiler calls and this platform does not provide.
//!
//! Everything here is an `extern "C"` symbol emitted by rustc's own code generation,
//! not an API an application names: the `__atomic_*` family that `core::sync::atomic`
//! lowers to on a CPU with no `LDREX`, `__sync_synchronize`, and `memcmp`/`bcmp`.
//! Linking this crate is what makes the target's `max-atomic-width: 32` and
//! `atomic-cas: true` true, and therefore what makes `AtomicU32`, `Arc` and
//! `symbian_std::sync` exist at all.
//!
//! # Why it is Rust and not C++
//!
//! The SDK's C++ shim exists for one reason: a leave is a real C++ exception here, and
//! only C++ can `TRAP` it (`shims/common/symrs_shim.h`). Nothing in this crate leaves —
//! `RFastLock::CreateLocal`, `Wait` and `Signal` are non-leaving euser members, and the
//! member ABI is ordinary AAPCS with `this` as argument 0 (observed, experiment 78) —
//! so there is nothing for C++ to do, and being C++ would only mean a second toolchain
//! in the way of code rustc can emit itself.
//!
//! # The one rule for everything in here
//!
//! **No code in this crate may perform an atomic operation.** These functions are what
//! an atomic operation compiles into, so one would call back into itself for ever.
//! Shared state uses [`cell::Shared`] with the ordering argument written out, the
//! accesses are volatile reads and writes, and the one place that genuinely needs
//! atomicity — creating the lock — uses `User::LockedInc`, the one real atomic euser
//! exports, which needs no lock of its own.
#![no_std]

pub mod atomic;
mod cell;
pub mod cstring;
pub mod lock;

pub use lock::{symrs_atomic_init_status, symrs_atomic_lock_handle};
