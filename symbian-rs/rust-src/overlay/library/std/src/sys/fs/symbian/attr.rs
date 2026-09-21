//! `FileAttr`, `FileType` and `FilePermissions` over `RFs::Entry` and `TEntry`.
//!
//! `TEntry` is a large class whose layout this SDK has **not** observed beyond its
//! first two words: `iAtt` at offset 0 and `iSize` at offset 4 (experiment 79 measured
//! them). Storage of the measured `sizeof(TEntry)` is handed to euser's own constructor
//! and only those two words are ever read back, so nothing here depends on where the
//! name or the timestamps sit.
//!
//! That is also why there are no timestamps: `TEntry::iModified` is a `TTime` at an
//! offset nobody has measured, so `modified`, `accessed` and `created` are
//! `Unsupported` rather than a guess at a field.

use crate::fmt;
use crate::io;
use crate::sys::time::SystemTime;
use crate::sys::unsupported;
use symbian_sys::efsrv::{KENTRY_ATT_DIR, TENTRY_OFFSET_ATT, TENTRY_OFFSET_SIZE, TEntryStorage};

/// `KEntryAttReadOnly` (`f32file.h`).
const KENTRY_ATT_READ_ONLY: u32 = 0x0001;

#[derive(Clone)]
pub struct FileAttr {
    att: u32,
    size: u64,
}

impl FileAttr {
    /// What `RFs::Entry` filled in.
    pub(super) fn of_entry(entry: &TEntryStorage) -> Self {
        FileAttr {
            att: entry.word_at(TENTRY_OFFSET_ATT),
            size: u64::from(entry.word_at(TENTRY_OFFSET_SIZE)),
        }
    }

    /// What an open handle can say without asking the directory: its size, and that it
    /// is a file, because a directory cannot be opened as one.
    pub(super) fn of_open_file(size: u64) -> Self {
        FileAttr { att: 0, size }
    }

    pub fn size(&self) -> u64 {
        self.size
    }

    pub fn perm(&self) -> FilePermissions {
        FilePermissions { readonly: self.att & KENTRY_ATT_READ_ONLY != 0 }
    }

    pub fn file_type(&self) -> FileType {
        FileType { directory: self.att & KENTRY_ATT_DIR != 0 }
    }

    pub fn modified(&self) -> io::Result<SystemTime> {
        unsupported()
    }

    pub fn accessed(&self) -> io::Result<SystemTime> {
        unsupported()
    }

    pub fn created(&self) -> io::Result<SystemTime> {
        unsupported()
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct FilePermissions {
    readonly: bool,
}

impl FilePermissions {
    pub fn readonly(&self) -> bool {
        self.readonly
    }

    /// Remembered, and then refused by [`super::set_perm`].
    ///
    /// Symbian's read-only bit is `KEntryAttReadOnly`, set with `RFs::SetAtt`, which
    /// this SDK has not declared because nothing has observed it. Recording the wish
    /// here and failing at the call is better than failing at the setter, which is
    /// where `std` gives no way to report anything.
    pub fn set_readonly(&mut self, readonly: bool) {
        self.readonly = readonly;
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct FileType {
    directory: bool,
}

impl FileType {
    pub fn is_dir(&self) -> bool {
        self.directory
    }

    pub fn is_file(&self) -> bool {
        !self.directory
    }

    /// Symbian 9.3 has no symbolic links at all, on any of its file systems.
    pub fn is_symlink(&self) -> bool {
        false
    }
}

/// `std::fs::FileTimes`: nothing can be set, because no timestamp field of `TEntry` has
/// been observed. [`super::set_times`] refuses.
#[derive(Copy, Clone, Debug, Default)]
pub struct FileTimes {}

impl FileTimes {
    pub fn set_accessed(&mut self, _t: SystemTime) {}
    pub fn set_modified(&mut self, _t: SystemTime) {}
}

/// `std::fs::ReadDir`: `RDir`'s `CDir` entry points leave and would need a C++ shim,
/// which step 77 did not add, so the directory iterator does not exist rather than
/// pretending an empty directory.
pub struct ReadDir(pub(super) !);

impl fmt::Debug for ReadDir {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0
    }
}

impl Iterator for ReadDir {
    type Item = io::Result<DirEntry>;

    fn next(&mut self) -> Option<io::Result<DirEntry>> {
        self.0
    }
}

pub struct DirEntry(pub(super) !);

impl DirEntry {
    pub fn path(&self) -> crate::path::PathBuf {
        self.0
    }

    pub fn file_name(&self) -> crate::ffi::OsString {
        self.0
    }

    pub fn metadata(&self) -> io::Result<FileAttr> {
        self.0
    }

    pub fn file_type(&self) -> io::Result<FileType> {
        self.0
    }
}
