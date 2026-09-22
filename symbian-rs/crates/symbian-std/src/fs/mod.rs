//! `std::fs` for Symbian: [`File`], [`OpenOptions`], [`Metadata`] and the free
//! functions, over `RFs` and `RFile`.
//!
//! # Paths
//!
//! A path is a `&str`, and it is a **Symbian** path. There is no `Path`, no `PathBuf`
//! and no `AsRef<Path>`: inventing one would imply POSIX rules this file system does
//! not have. What it does have:
//!
//! | | |
//! |---|---|
//! | Drive letter | `C:\` internal disk, `E:\` memory card, `Z:\` read-only ROM. There is no root above the drives and no mount point |
//! | Separator | a backslash, so a Rust literal is `"E:\\symdev\\notes.txt"` |
//! | Length | at most `KMaxFileName` = 256 code units, or `KErrOverflow` before any call is made |
//! | Relative paths | resolved against the *session* path, which belongs to the file-server session and not to the process's working directory — there is no `current_dir` |
//! | `\private\<uid3>\` | data caging: a process reaches its own private directory and, without the `AllFiles` capability, no other. The file server enforces it; no permission bit is involved |
//!
//! **EKA2L1 is more permissive than a phone.** Experiment 78 put `Z:\`, `Y:\`, `Q:\`
//! and a `*` inside a path component through the emulator's file server and every one
//! was accepted. An emulator run is never evidence that a device would take a path.
//!
//! # What is not here
//!
//! `copy`, `remove_dir_all`, `set_permissions`, `canonicalize`, `hard_link`,
//! `soft_link`: each needs either a type this SDK has not observed or a concept
//! Symbian does not have. None of them is faked.
//!
//! # `read_dir` borrows, where `std`'s owns
//!
//! ```ignore
//! for entry in &fs::read_dir("E:\\symdev")? {
//!     if entry.is_file() && entry.name() == "notes.txt" { … }
//! }
//! ```
//!
//! Note the `&`, and that an entry is not a `Result`. `std::fs::read_dir` yields owned
//! entries — an `OsString` and a `PathBuf` each, two heap cells per entry — and C++
//! yields a reference into the one `CDir` the file server filled, with none. This
//! shape is the C++ one: the directory is read in a single `RFs::GetDir`, and every
//! entry and name borrows from it, so listing costs exactly the allocation the C++
//! costs. The price is the `&`: an iterator cannot hand out entries that borrow from
//! itself, so it iterates over the directory rather than consuming it. Nothing can
//! fail after `read_dir` returns, which is why the items are not `Result`s.
mod file;
mod metadata;
mod open_options;

pub use file::File;
pub use metadata::Metadata;
pub use open_options::OpenOptions;
pub use symbian_core::fs::{DirEntry, Name};

/// A directory's entries, as [`read_dir`] returns them.
pub type ReadDir = symbian_core::fs::Dir;

use alloc::vec::Vec;
use symbian_core::ErrorKind as SymKind;
use symbian_core::fs::{Entry, ProcessSession};

use crate::io::{Read, Result, Write};

/// Creates a directory and every missing parent, as `std::fs::create_dir_all`.
///
/// An existing directory is success, as in `std` — Symbian's `RFs::MkDirAll` reports
/// `KErrAlreadyExists` for it, and that is translated here rather than passed on.
///
/// `RFs::MkDirAll` treats the last component of its argument as a file name and does
/// not create it, so a trailing backslash is added when the caller left it out.
pub fn create_dir_all(path: &str) -> Result<()> {
    match ProcessSession::make_dirs(path) {
        Err(e) if e.kind() == SymKind::AlreadyExists => Ok(()),
        other => Ok(other?),
    }
}

/// Reads a directory, as `std::fs::read_dir` — in one `RFs::GetDir` call, with no
/// allocation per entry. Iterate it by reference: `for entry in &fs::read_dir(path)?`.
/// See [the module documentation](self) for why it differs from `std` there.
pub fn read_dir(path: &str) -> Result<ReadDir> {
    Ok(ReadDir::read(path)?)
}

/// Removes a file, as `std::fs::remove_file` (`RFs::Delete`).
///
/// Symbian is stricter than POSIX: a file another handle still has open is
/// `KErrInUse`, not a deferred unlink.
pub fn remove_file(path: &str) -> Result<()> {
    Ok(ProcessSession::delete(path)?)
}

/// Renames a file or directory, as `std::fs::rename` (`RFs::Rename`).
///
/// Symbian is stricter than POSIX here too: if `to` already exists this is
/// `KErrAlreadyExists`, where `std::fs::rename` replaces the destination silently.
pub fn rename(from: &str, to: &str) -> Result<()> {
    Ok(ProcessSession::rename(from, to)?)
}

/// What the file server knows about one entry, as `std::fs::metadata` (`RFs::Entry`).
pub fn metadata(path: &str) -> Result<Metadata> {
    Ok(Metadata::of_entry(&Entry::of(path)?))
}

/// Reads a whole file, as `std::fs::read`.
pub fn read(path: &str) -> Result<Vec<u8>> {
    let mut file = File::open(path)?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;
    Ok(bytes)
}

/// Writes a whole file, replacing what was there, as `std::fs::write`.
pub fn write(path: &str, contents: &[u8]) -> Result<()> {
    File::create(path)?.write_all(contents)
}
