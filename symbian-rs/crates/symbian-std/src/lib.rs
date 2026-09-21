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
//! Two shapes, and [`macro@main`] is the same line in both.
//!
//! **With `std`** (design spec §11 step 77), which is what a new application should
//! use: there is no `#![no_std]`, no `extern crate alloc` and no allocator or panic
//! handler to install, because a real `std` for `target_os = "symbian"` brings all of
//! them. The manifest says `[language] name = "rust-std"`, this crate is depended on
//! with `default-features = false, features = ["std"]`, and what is left of it is the
//! entry attribute, the prelude and [`test_report`] — everything else `std` itself
//! does better.
//!
//! ```ignore
//! use std::fs::File;
//!
//! #[symbian_std::main]
//! fn main() -> std::io::Result<()> {
//!     Ok(())
//! }
//! ```
//!
//! **Without it**, which is still supported and still smaller — a `#![no_std]` hello
//! is 3 187 bytes against a `std` one's 52 206:
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
//! [`macro@main`] writes the `E32Main()` `eexe.lib` calls, in both shapes; there is no
//! `#![no_main]` and no entry macro to remember, because the crate is compiled as a
//! `staticlib` and rustc never looks for a `main` of its own.
//!
//! [`test_report`] is how an example says whether it passed, in a file
//! `symdev test --emulator` can read back off the emulated drive.
//!
//! What follows is the `#![no_std]` half of the crate, and none of it is built under
//! the `std` feature.
#![no_std]
// Everything an application touches is safe, and the modules that make up the file and
// I/O facade say so with their own `#![forbid(unsafe_code)]`. [`sync`] and [`thread`]
// are the exception CLAUDE.md names: a mutex and a thread are built out of kernel
// handles and raw pointers, and there is no layer below them to hide that in. Every
// `unsafe` block in this crate lives in those two modules and carries a `// SAFETY:`
// note.
#![deny(unsafe_code)]

extern crate alloc;
/// With a real `std` for this target there is no facade to build: `std::fs`,
/// `std::io`, `std::sync`, `std::thread` and `std::time` are the genuine articles, and
/// the modules below would be a second, subtly different copy of each. Worse than
/// different — *wrong*: `symbian_std::thread::spawn` and `std::thread::spawn` each
/// switch heap serialisation on behind their own flag, and a program that used both
/// would have two locks over one heap.
///
/// So under this feature the crate is only what `std` has no answer for: the entry
/// point attribute, the prelude and [`test_report`].
#[cfg(feature = "std")]
extern crate std;

#[cfg(not(feature = "std"))]
pub mod fs;
#[cfg(not(feature = "std"))]
pub mod io;
pub mod locale;
#[cfg(not(feature = "std"))]
pub mod net;
pub mod prelude;
#[cfg(not(feature = "std"))]
#[allow(unsafe_code)]
pub mod sync;
pub mod test_report;
#[cfg(not(feature = "std"))]
#[allow(unsafe_code)]
pub mod thread;
#[cfg(not(feature = "std"))]
pub mod time;

#[cfg(not(feature = "std"))]
pub use symbian_async as task;
/// An Avkon application: the [`ui::App`] trait, the drawing context and the keys the
/// C++ shim forwards (design spec §11 step 75). It is a module here, rather than a
/// crate an application names itself, so that a program's first line stays one `use`.
///
/// It is the one part of this facade with no `std` analogue at all, and the crate's
/// own documentation says why: an Avkon application is not a `fn main` running to
/// completion, but a framework that owns the event loop and calls into the program.
#[cfg(not(feature = "std"))]
pub use symbian_ui as ui;

pub use symbian_macros::main;
/// What a `fn main` may return, and the `TInt` it becomes. An application implements
/// [`IntoExitCode`] for its own error type to return it from `main`; `()`, `i32`,
/// `SymbianError`, `io::Error` and any `Result` of those are already covered.
#[cfg(feature = "runtime")]
pub use symbian_runtime::{ExitCode, IntoExitCode};

/// What `#[symbian_std::main]` calls. Not part of the API an application writes.
///
/// There are two of it, one per shape of the SDK, and the attribute expands to the same
/// line for both so that a program moving from `#![no_std]` to `std` changes nothing
/// but its `use` lines.
#[cfg(feature = "runtime")]
#[doc(hidden)]
pub fn __start<T: IntoExitCode>(main: fn() -> T) -> i32 {
    symbian_runtime::start(main)
}

/// The `std` shape: `std`'s own runtime start-up, which initialises the runtime, runs
/// `main`, converts its `Termination` to an exit code, flushes `stdout` and runs the
/// main thread's thread-local destructors — the last of which nothing in this kernel
/// does by itself (experiment 88).
#[cfg(feature = "std")]
#[doc(hidden)]
pub fn __start<T: std::process::Termination + 'static>(main: fn() -> T) -> i32 {
    std::os::symbian::start(main)
}
