//! `fs::OpenOptions`: `std`'s builder over the three `RFile` opening calls.
use symbian_core::fs::{File as SymFile, FileMode, Seek as SymSeek, with_session};

use super::File;
use crate::io::{Error, ErrorKind, Result};

/// `std::fs::OpenOptions`, with `std`'s builder methods and `std`'s rules about which
/// combinations are valid.
///
/// Symbian has three opening calls and this type chooses between them:
/// `RFile::Open` for an existing file, `RFile::Create` for one that must not exist, and
/// `RFile::Replace` for create-or-truncate.
#[derive(Debug, Clone, Copy, Default)]
pub struct OpenOptions {
    read: bool,
    write: bool,
    append: bool,
    truncate: bool,
    create: bool,
    create_new: bool,
}

impl OpenOptions {
    /// A builder with every option off, as `std::fs::OpenOptions::new`.
    pub const fn new() -> Self {
        Self {
            read: false,
            write: false,
            append: false,
            truncate: false,
            create: false,
            create_new: false,
        }
    }

    /// Open for reading.
    pub const fn read(mut self, read: bool) -> Self {
        self.read = read;
        self
    }

    /// Open for writing.
    pub const fn write(mut self, write: bool) -> Self {
        self.write = write;
        self
    }

    /// Open for writing at the end of the file.
    ///
    /// **This is where Symbian differs and the difference is real.** `std`'s `append`
    /// is POSIX `O_APPEND`: every write goes to the end of the file as one atomic step,
    /// whatever another writer did in between. `RFile` has no such mode, so this seeks
    /// to the end **once, when the file is opened**, and writes from there. With one
    /// writer the two behave the same; with two, this one can overwrite.
    pub const fn append(mut self, append: bool) -> Self {
        self.append = append;
        self
    }

    /// Truncate the file to zero when it is opened.
    pub const fn truncate(mut self, truncate: bool) -> Self {
        self.truncate = truncate;
        self
    }

    /// Create the file if it is not there.
    pub const fn create(mut self, create: bool) -> Self {
        self.create = create;
        self
    }

    /// Create the file and fail if it is already there (`RFile::Create`).
    pub const fn create_new(mut self, create_new: bool) -> Self {
        self.create_new = create_new;
        self
    }

    /// The file mode the combination asks for, or [`ErrorKind::InvalidInput`] for a
    /// combination `std` also rejects: no access at all, or a creating or truncating
    /// option without write access.
    fn mode(&self) -> Result<FileMode> {
        let writing = self.write || self.append;
        if (self.create || self.create_new || self.truncate) && !writing {
            return Err(Error::from(ErrorKind::InvalidInput));
        }
        match (self.read, writing) {
            (true, true) => Ok(FileMode::ReadWrite),
            (false, true) => Ok(FileMode::Write),
            (true, false) => Ok(FileMode::Read),
            (false, false) => Err(Error::from(ErrorKind::InvalidInput)),
        }
    }

    /// Opens `path` with these options, as `std::fs::OpenOptions::open`.
    pub fn open(&self, path: &str) -> Result<File> {
        let mode = self.mode()?;
        let inner = with_session(|fs| self.open_inner(fs, path, mode))?;
        let file = File::of(inner);
        if self.append {
            let mut file = file;
            file.seek_to_end()?;
            return Ok(file);
        }
        Ok(file)
    }

    /// Which of the three `RFile` calls this combination is, and the fix-ups the ones
    /// Symbian does not have directly need.
    fn open_inner(
        &self,
        fs: &mut symbian_core::FileServer,
        path: &str,
        mode: FileMode,
    ) -> symbian_core::Result<SymFile> {
        if self.create_new {
            return SymFile::create_new(fs, path, mode);
        }
        if self.create && self.truncate {
            return SymFile::replace(fs, path, mode);
        }
        if self.create {
            // Symbian has no create-if-missing-keep-contents: `Open` says whether the
            // file is there, and `Create` makes it when it is not.
            return match SymFile::open(fs, path, mode) {
                Err(e) if e.kind() == symbian_core::ErrorKind::NotFound => {
                    SymFile::create_new(fs, path, mode)
                }
                other => other,
            };
        }
        let mut file = SymFile::open(fs, path, mode)?;
        if self.truncate {
            file.set_size(0)?;
            file.seek(SymSeek::Start(0))?;
        }
        Ok(file)
    }
}
