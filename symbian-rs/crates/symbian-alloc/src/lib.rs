//! The Rust heap on Symbian OS 9.3: a [`GlobalAlloc`](core::alloc::GlobalAlloc) over
//! `User::Alloc` / `User::Free` / `User::ReAlloc` (design spec §6, experiment 68).
//!
//! The crate holds the allocator *type* only. The one `#[global_allocator]` static lives
//! in `symbian-runtime`, which every application already links, so an application never
//! has to remember to install a heap and no two crates can install two.
#![no_std]

mod heap;
pub mod serialise;

pub use heap::{KERR_NO_MEMORY, MAX_TRUSTED_ALIGN, SymbianHeap};
pub use serialise::{is_serialised, serialise_across_threads};
