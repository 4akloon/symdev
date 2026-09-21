//! `std::fs` over the Symbian file server (`RFs`, `RFile`).
//!
//! This is experiment 79's `symbian_std::fs` re-hosted, which is what the design spec
//! said would happen: it was written in `std`'s shape from the start, so it moves
//! rather than being rewritten.
//!
//! # What is real, and what is `Unsupported`
//!
//! Real: opening (all six `OpenOptions` combinations), reading, writing, seeking,
//! flushing, truncating, `metadata`, `exists`, `remove_file`, `rename`, `create_dir`
//! and `create_dir_all`.
//!
//! `Unsupported`, and each for a stated reason rather than because it was skipped:
//!
//! | | why |
//! |---|---|
//! | `read_dir` | `RDir`/`CDir`'s `NewL`/`AddL` **leave**, so they need a C++ `TRAP` shim that step 77 did not add |
//! | `symlink`, `read_link`, `hard_link` | Symbian 9.3 has neither, on any file system |
//! | `set_permissions` | `RFs::SetAtt` has not been observed, so the SDK does not declare it |
//! | `set_times` | no timestamp field of `TEntry` has had its offset measured |
//! | `canonicalize` | there is no root above the drives and no working directory to resolve against: a relative path is resolved against the *session* path, which belongs to the file-server session |
//!
//! # Paths are Symbian paths
//!
//! A `Path` here names a drive (`C:\`, `E:\`, `Z:\`), separates with a backslash, is at
//! most `KMaxFileName` = 256 code units, and under `\private\<uid3>` is data-caged by
//! the file server rather than by a permission bit. `sys::path` uses the backslash
//! separator for this target, but does **not** parse the drive letter as a prefix, so
//! `Path::is_absolute` answers `false` for `C:\x`; that is a gap and is named here
//! rather than papered over.

mod attr;
mod file;
mod session;

pub use attr::{DirEntry, FileAttr, FilePermissions, FileTimes, FileType, ReadDir};
pub use crate::sys::fs::common::Dir;
pub use file::{File, OpenOptions};

use crate::io;
use crate::path::{Path, PathBuf};
use crate::sys::unsupported;
use file::check;
use session::with_session;
use symbian_sys::efsrv::{
    RFs_Att, RFs_Delete, RFs_Entry, RFs_MkDir, RFs_Rename, TEntryStorage, TEntry_ctor,
};

use crate::sys::pal::symbian::des::PathBuf16;

/// The path as the descriptor every efsrv call takes.
fn native(path: &Path) -> io::Result<PathBuf16> {
    let path = path.to_str().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidFilename,
            "a Symbian path is UTF-16 and is built from a UTF-8 Rust path",
        )
    })?;
    PathBuf16::new(path)
}

#[derive(Debug)]
pub struct DirBuilder {}

impl DirBuilder {
    pub fn new() -> DirBuilder {
        DirBuilder {}
    }

    /// `RFs::MkDir`, which creates the last component only.
    ///
    /// Symbian names a directory with a trailing backslash; without one the file server
    /// takes the last component as a file name and creates its *parent*. One is added
    /// here so that `create_dir("E:\\a\\b")` makes `b` and not just `a`.
    pub fn mkdir(&self, path: &Path) -> io::Result<()> {
        let path = native(&with_trailing_separator(path))?;
        with_session(|session| {
            // SAFETY: a non-leaving efsrv member taking `this` as argument 0; the
            // descriptor is borrowed for the call and only read.
            check(unsafe { RFs_MkDir(session.as_rfs(), path.as_tdesc16()) })
        })
    }
}

fn with_trailing_separator(path: &Path) -> PathBuf {
    let mut owned = path.to_path_buf();
    if !path.as_os_str().as_encoded_bytes().ends_with(b"\\") {
        owned.push("");
    }
    owned
}

pub fn readdir(_path: &Path) -> io::Result<ReadDir> {
    unsupported()
}

pub fn unlink(path: &Path) -> io::Result<()> {
    let path = native(path)?;
    with_session(|session| {
        // SAFETY: as `DirBuilder::mkdir`.
        check(unsafe { RFs_Delete(session.as_rfs(), path.as_tdesc16()) })
    })
}

pub fn rename(old: &Path, new: &Path) -> io::Result<()> {
    let old = native(old)?;
    let new = native(new)?;
    with_session(|session| {
        // SAFETY: as `DirBuilder::mkdir`, with two borrowed descriptors.
        check(unsafe { RFs_Rename(session.as_rfs(), old.as_tdesc16(), new.as_tdesc16()) })
    })
}

pub fn set_perm(_path: &Path, _perm: FilePermissions) -> io::Result<()> {
    unsupported()
}

pub fn set_perm_nofollow(_path: &Path, _perm: FilePermissions) -> io::Result<()> {
    unsupported()
}

pub fn set_times(_path: &Path, _times: FileTimes) -> io::Result<()> {
    unsupported()
}

pub fn set_times_nofollow(_path: &Path, _times: FileTimes) -> io::Result<()> {
    unsupported()
}

/// `RFs::RmDir` has not been observed, so removing a directory is refused rather than
/// aimed at `RFs::Delete`, which is for files and would report a confusing code.
pub fn rmdir(_path: &Path) -> io::Result<()> {
    unsupported()
}

pub fn remove_dir_all(_path: &Path) -> io::Result<()> {
    unsupported()
}

/// `RFs::Att`, which asks only for the attribute word and is the cheapest question the
/// file server answers about a name.
pub fn exists(path: &Path) -> io::Result<bool> {
    let path = native(path)?;
    let mut att = 0u32;
    with_session(|session| {
        // SAFETY: a non-leaving const member taking `this` as argument 0; `att` is an
        // owned local the file server writes one word into.
        match unsafe { RFs_Att(session.as_rfs(), path.as_tdesc16(), &mut att) } {
            0 => Ok(true),
            // KErrNotFound, KErrPathNotFound
            -1 | -12 => Ok(false),
            code => Err(io::Error::from_raw_os_error(code)),
        }
    })
}

pub fn readlink(_path: &Path) -> io::Result<PathBuf> {
    unsupported()
}

pub fn symlink(_original: &Path, _link: &Path) -> io::Result<()> {
    unsupported()
}

pub fn link(_src: &Path, _dst: &Path) -> io::Result<()> {
    unsupported()
}

pub fn stat(path: &Path) -> io::Result<FileAttr> {
    let path = native(path)?;
    let mut entry = TEntryStorage::zeroed();
    with_session(|session| {
        // SAFETY: `TEntry`'s own constructor is run over storage of the measured
        // `sizeof(TEntry)` before the file server fills it, and `RFs::Entry` is a
        // non-leaving const member taking `this` as argument 0.
        unsafe {
            TEntry_ctor(entry.as_entry());
            check(RFs_Entry(session.as_rfs(), path.as_tdesc16(), entry.as_entry()))?;
        }
        Ok(FileAttr::of_entry(&entry))
    })
}

/// There are no symbolic links, so this is `stat`.
pub fn lstat(path: &Path) -> io::Result<FileAttr> {
    stat(path)
}

pub fn canonicalize(_path: &Path) -> io::Result<PathBuf> {
    unsupported()
}

/// The generic byte-for-byte copy `std` provides for platforms with no `copy_file_range`
/// equivalent. `RFs` has no server-side copy at all on 9.3.
pub fn copy(from: &Path, to: &Path) -> io::Result<u64> {
    crate::sys::fs::common::copy(from, to)
}
