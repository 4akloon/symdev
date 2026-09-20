//! `fs::File`: `std`'s shape over `RFile`.
use symbian_core::fs::{File as SymFile, Seek as SymSeek};

use super::{Metadata, OpenOptions};
use crate::io::{Error, ErrorKind, Read, Result, Seek, SeekFrom, Write};

/// An open file, as `std::fs::File`.
///
/// The handle closes when the value is dropped, and closing commits what the file
/// server still holds.
///
/// Where this differs from `std`: the methods that change the file take `&mut self`
/// rather than `&self`. `std` can take `&self` because a POSIX descriptor is shared
/// state behind the kernel; here `RFile::SetSize` and `RFile::Write` are non-`const`
/// members of the handle, and the borrow checker says so rather than hiding it.
pub struct File {
    inner: SymFile,
}

impl File {
    pub(crate) const fn of(inner: SymFile) -> Self {
        Self { inner }
    }

    /// Opens an existing file for reading (`RFile::Open`).
    pub fn open(path: &str) -> Result<Self> {
        OpenOptions::new().read(true).open(path)
    }

    /// Creates a file, truncating it if it is already there (`RFile::Replace`).
    pub fn create(path: &str) -> Result<Self> {
        OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)
    }

    /// Creates a file and fails if it already exists (`RFile::Create`).
    pub fn create_new(path: &str) -> Result<Self> {
        OpenOptions::new().write(true).create_new(true).open(path)
    }

    /// A builder, as `std::fs::File::options`.
    pub const fn options() -> OpenOptions {
        OpenOptions::new()
    }

    /// The size of the open file, as `std::fs::File::metadata`.
    ///
    /// It asks the handle (`RFile::Size`) rather than the directory, so it needs no
    /// session and sees what has been written through this handle.
    pub fn metadata(&self) -> Result<Metadata> {
        Ok(Metadata::of_open_file(self.inner.size()?))
    }

    /// Truncates or extends the file (`RFile::SetSize`).
    pub fn set_len(&mut self, size: u64) -> Result<()> {
        Ok(self.inner.set_size(size)?)
    }

    /// Commits the data and the directory entry to the medium (`RFile::Flush`), as
    /// `std::fs::File::sync_all`.
    ///
    /// `sync_data` has no separate call here: `RFile::Flush` writes both, so offering
    /// a cheaper one would be a promise Symbian does not make.
    pub fn sync_all(&mut self) -> Result<()> {
        Ok(self.inner.flush()?)
    }

    /// Moves to the end, for [`OpenOptions::append`].
    pub(crate) fn seek_to_end(&mut self) -> Result<()> {
        self.inner.seek(SymSeek::End(0))?;
        Ok(())
    }
}

impl Read for File {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        Ok(self.inner.read(buf)?)
    }
}

impl Write for File {
    /// `RFile::Write` writes the whole descriptor or fails, so this never reports a
    /// short write and [`Write::write_all`] never has to loop. That is Symbian being
    /// stricter than `std`, which is the safe direction.
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        self.inner.write(buf)?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> Result<()> {
        Ok(self.inner.flush()?)
    }
}

impl Seek for File {
    /// `RFile::Seek` counts in a `TInt`, so a position or an offset that does not fit
    /// 31 bits is [`ErrorKind::FileTooLarge`] rather than a silently wrapped number.
    /// The file server's own size is 32 bits; this SDK does not pretend otherwise.
    fn seek(&mut self, pos: SeekFrom) -> Result<u64> {
        let to = match pos {
            SeekFrom::Start(n) => SymSeek::Start(offset(i64::try_from(n).unwrap_or(i64::MAX))?),
            SeekFrom::End(n) => SymSeek::End(offset(n)?),
            SeekFrom::Current(n) => SymSeek::Current(offset(n)?),
        };
        Ok(self.inner.seek(to)?)
    }
}

/// A `TInt` offset, or [`ErrorKind::FileTooLarge`] for one that does not fit.
fn offset(n: i64) -> Result<i32> {
    i32::try_from(n).map_err(|_| Error::from(ErrorKind::FileTooLarge))
}
