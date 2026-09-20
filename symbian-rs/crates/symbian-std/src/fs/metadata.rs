//! `fs::Metadata`: what the file server knows about one entry.
use symbian_core::Entry;

/// `std::fs::Metadata`, as much of it as Symbian answers today.
///
/// `permissions()`, `modified()`, `created()` and `accessed()` are absent rather than
/// faked. The `TEntry` behind this does carry `iModified`, but there is no `SystemTime`
/// in this SDK to return it as, and Symbian's read-only bit is one attribute rather
/// than a `Permissions` set; both wait for the step that can answer them honestly.
/// `is_symlink()` is absent because this file system has no symbolic links at all.
#[derive(Debug, Clone, Copy)]
pub struct Metadata {
    len: u64,
    attributes: u32,
    is_dir: bool,
    is_file: bool,
}

impl Metadata {
    /// The size in bytes, as `std::fs::Metadata::len`.
    pub const fn len(&self) -> u64 {
        self.len
    }

    /// Whether the entry holds no bytes. `std` has no such method on `Metadata`;
    /// clippy asks for one wherever there is a `len`, and it says something true.
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Whether the entry is a directory.
    pub const fn is_dir(&self) -> bool {
        self.is_dir
    }

    /// Whether the entry is an ordinary file — not a directory and not a volume label.
    pub const fn is_file(&self) -> bool {
        self.is_file
    }

    /// The raw `KEntryAtt*` bits. Symbian's own answer, for code that needs more than
    /// the two questions `std` asks.
    pub const fn attributes(&self) -> u32 {
        self.attributes
    }

    /// The metadata of a directory entry the file server described.
    pub(crate) fn of_entry(entry: &Entry) -> Self {
        Self {
            len: entry.size(),
            attributes: entry.attributes(),
            is_dir: entry.is_dir(),
            is_file: entry.is_file(),
        }
    }

    /// The metadata of an open file, which is a file by construction and whose size
    /// comes from the handle rather than from a second look at the directory.
    pub(crate) const fn of_open_file(len: u64) -> Self {
        Self {
            len,
            attributes: 0,
            is_dir: false,
            is_file: true,
        }
    }
}
