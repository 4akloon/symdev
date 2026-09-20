//! `std`'s module tree for Symbian OS 9.3, so that porting a program is changing its
//! `use` lines (design spec §6a, §11 step 71).
//!
//! An application developer should almost never touch a Symbian-specific API. Where
//! Rust already has a name and a shape, this crate uses that name and that shape:
//!
//! ```ignore
//! use symbian_std::fs::File;
//! use symbian_std::io::{Read, Write};
//!
//! let mut file = File::create("E:\\symdev\\notes.txt")?;
//! file.write_all(b"hello")?;
//! ```
//!
//! Everything below it stays reachable — `symbian_core` for descriptors, `RFs`, `RFile`
//! and the raw `TInt`, `symbian_sys` for the exports themselves. Those are the escape
//! hatch, not the road.
//!
//! # Where Symbian differs, this crate says so
//!
//! Pretending would be a lie the first failure exposes, so the differences are in the
//! types and in the documentation rather than hidden:
//!
//! - **Paths are `&str`, and they are Symbian paths.** There is no `Path`, no `PathBuf`
//!   and no `AsRef<Path>`: a path names a drive (`C:\`, `E:\`, `Z:\`), separates with a
//!   backslash, is at most `KMaxFileName` = 256 code units, and under `\private\<uid3>`
//!   it is data-caged by the file server rather than by a permission bit. The rules are
//!   written out in [`fs`].
//! - **Errors keep their `TInt`.** [`io::Error::kind`] speaks `std`'s vocabulary and
//!   [`io::Error::raw_os_error`] hands back the exact Symbian code, so `?` works across
//!   an application and nothing is lost on the way.
//! - **No `std::error::Error` payloads.** `io::Error` is one `TInt`; there is no
//!   `Error::new(kind, source)` to attach a message to, because there is nowhere for
//!   the message to go on a phone with no stderr.
//! - **Capabilities, the Avkon event loop and access points have no `std` analogue**
//!   and are not faked here. They are declared in `symdev.toml` and handled by the
//!   crates that own them.
//!
//! # Where an application starts
//!
//! ```ignore
//! #![no_std]
//!
//! use symbian_std::prelude::*;
//!
//! #[symbian_std::main]
//! fn main() -> Result<()> {
//!     Ok(())
//! }
//! ```
//!
//! [`macro@main`] writes the `E32Main()` `eexe.lib` calls; there is no `#![no_main]`
//! and no entry macro to remember, because the crate is compiled as a `staticlib` and
//! rustc never looks for a `main` of its own. `#![no_std]` stays, and stays honest:
//! there is no `std` for this target.
//! [`sync`] and [`thread`] are step 72: `Arc`, `Mutex`, `Once` and `thread::spawn` over
//! the atomics the SDK's compiler-runtime archive provides on a CPU that has no atomic
//! instruction. Both modules say plainly what they cannot do — see their own
//! documentation before reaching for them, because every atomic operation on this
//! device is a kernel call.
//!
//! [`test_report`] is the other half of step 71: how an example says whether it passed,
//! in a file `symdev test --emulator` can read back off the emulated drive.
#![no_std]
// Everything an application touches is safe, and the modules that make up the file and
// I/O facade say so with their own `#![forbid(unsafe_code)]`. [`sync`] and [`thread`]
// are the exception CLAUDE.md names: a mutex and a thread are built out of kernel
// handles and raw pointers, and there is no layer below them to hide that in. Every
// `unsafe` block in this crate lives in those two modules and carries a `// SAFETY:`
// note.
#![deny(unsafe_code)]

extern crate alloc;

pub mod fs;
pub mod io;
pub mod prelude;
#[allow(unsafe_code)]
pub mod sync;
pub mod test_report;

pub use symbian_macros::main;
/// What a `fn main` may return, and the `TInt` it becomes. An application implements
/// [`IntoExitCode`] for its own error type to return it from `main`; `()`, `i32`,
/// `SymbianError`, [`io::Error`] and any `Result` of those are already covered.
pub use symbian_runtime::{ExitCode, IntoExitCode};
#[allow(unsafe_code)]
pub mod thread;
