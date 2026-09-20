//! Raw `extern "C"` declarations of the Symbian OS 9.3 user-side DLL exports the Rust
//! SDK calls without a C++ shim, plus the shim's own entry points. Every symbol's
//! mangled name is the one `nm -D` prints for `epoc32/release/armv5/lib/<dll>.dso`
//! (S60 3rd FP2) and is cited next to the declaration.
//!
//! What may be declared here, and what has to go through the C++ shim instead, is the
//! rule written out in `symbian-rs/shims/common/symrs_shim.h`. In short: **a call that
//! can leave is never declared here**, because a leave is a real C++ exception and the
//! whole Rust text is one `cantunwind` range; everything else may be, including
//! non-static member functions, whose calling convention (`this` as argument 0) was
//! observed in experiment 78 rather than assumed.
#![no_std]

pub mod des;
pub mod des16;
pub mod des8;
pub mod efsrv;
pub mod esock;
pub mod euser;
pub mod libcalls;
pub mod shim;
pub mod thread;
pub mod time;
