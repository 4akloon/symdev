//! Raw `extern "C"` declarations of the Symbian OS 9.3 user-side DLL exports the Rust SDK
//! calls without a C++ shim: static member functions and free functions, which are plain
//! ARM EABI calls. One module per DLL; every symbol's mangled name is the one `nm -D`
//! prints for `epoc32/release/armv5/lib/<dll>.dso` (S60 3rd FP2) and is cited next to the
//! declaration. Nothing that can leave is declared here (design spec §7).
#![no_std]

pub mod des;
pub mod euser;
