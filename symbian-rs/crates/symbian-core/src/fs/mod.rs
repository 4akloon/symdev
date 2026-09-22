//! The Symbian file system as domain types: a session ([`FileServer`]), an open file
//! ([`File`]) and a directory entry ([`Entry`]) — design spec §7, §11 step 71.
//!
//! **Nothing here needs the C++ shim.** `f32file.h` declares no leaving member on
//! `RFs`, `RFile`, `RDir` or `TEntry`; every one of them reports failure as a `TInt`,
//! so Rust calls them directly with `this` as argument 0 (the member ABI observed in
//! experiment 78). The one shim call in this module, [`FileServer::ensure_path_exists`],
//! is `BaflUtils::EnsurePathExistsL` from bafl.dso, which is not part of the file API
//! and does leave.
//!
//! This is the lower level. An application uses `symbian-std`'s `fs` and `io`, which
//! are `std`'s names and shapes over these types.
//!
//! ## Paths are Symbian paths, and this module does not pretend otherwise
//!
//! A path is a `&str` at the surface, but it is a *Symbian* path and no amount of
//! wrapping makes it a POSIX one:
//!
//! - It names a **drive letter**: `C:\` is the phone's internal disk, `E:\` the memory
//!   card, `Z:\` the read-only ROM. There is no root above the drives and no mount
//!   point; a path with no drive is resolved against the session path.
//! - The separator is a **backslash**, so a Rust literal needs `"E:\\symdev\\out.txt"`.
//! - `\private\<uid3>\` is **data caging**: a process may open its own private
//!   directory and, without `AllFiles`, nobody else's. That is platform security, not a
//!   permission bit, and it is enforced by the file server.
//! - `KMaxFileName` is `0x100` code units ([`MAX_FILE_NAME`]); a longer path is
//!   `KErrOverflow` before any call is made.
//!
//! EKA2L1 is more permissive than a phone about all of this — experiment 78 put `Z:\`,
//! `Y:\`, `Q:\` and a `*` inside a path component through the emulator's file server
//! and every one succeeded — so an emulator run is never evidence that a device would
//! accept a path.
mod dir;
mod entry;
mod file;
mod request;
mod server;
mod session;

pub use dir::{Dir, DirEntry, Iter, Name};
pub use entry::Entry;
pub use file::{File, FileMode, Seek};
pub use request::Opening;
pub use server::FileServer;
pub use session::{ProcessSession, with_session};

/// `KMaxFileName` (`e32const.h` line 390: `const TInt KMaxFileName=0x100;`): the longest
/// path the file server accepts, and the size of the descriptor a path is built into.
///
/// A path is a `&str` in the application and a `Buf16<MAX_FILE_NAME>` by the time it
/// reaches the file server: `KErrOverflow` if it is longer, `KErrArgument` if it is not
/// valid UTF-16 — the same codes every other conversion in this crate reports. The
/// buffer is always built in the frame that makes the call: returned by value it was a
/// 516-byte `memcpy` at every caller (experiment 105).
pub const MAX_FILE_NAME: usize = 0x100;
