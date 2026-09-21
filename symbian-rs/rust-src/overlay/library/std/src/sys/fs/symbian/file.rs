//! `File` and `OpenOptions` over `RFile` (experiment 79, re-hosted).
//!
//! Every `RFile` member used here is non-leaving, so Rust calls it directly with `this`
//! as argument 0 — the member ABI observed in experiment 78, not assumed.
//!
//! # `&self` where Symbian says `RFile*`
//!
//! `std::fs::File` takes `&self` for `write` and `seek`, because a POSIX descriptor is
//! shared state behind the kernel. `RFile::Write` and `RFile::SetSize` are non-`const`
//! C++ members. The handle is one `TInt` the kernel indexes with, so a `&self` call
//! changes nothing on this side; the `UnsafeCell` below is what makes that explicit
//! rather than a cast that hides it.

use crate::sys::pal::symbian::des::{Bytes, BytesMut, PathBuf16};
use super::attr::FileAttr;
use super::session::with_session;
use crate::cell::UnsafeCell;
use crate::fs::TryLockError;
use crate::io::{self, BorrowedCursor, SeekFrom};
use crate::sys::{unsupported, unsupported_err};
use symbian_sys::efsrv::{
    EFILE_READ, EFILE_SHARE_ANY, EFILE_WRITE, ESEEK_CURRENT, ESEEK_END, ESEEK_START, RFile,
    RFile_Create, RFile_Flush, RFile_Open, RFile_Read, RFile_Replace, RFile_Seek, RFile_SetSize,
    RFile_Size, RFile_Write,
};
use symbian_sys::euser::{RHandleBase, RHandleBase_Close};

#[derive(Clone, Debug, Default)]
pub struct OpenOptions {
    read: bool,
    write: bool,
    append: bool,
    truncate: bool,
    create: bool,
    create_new: bool,
}

impl OpenOptions {
    pub fn new() -> OpenOptions {
        OpenOptions::default()
    }
    pub fn read(&mut self, read: bool) {
        self.read = read;
    }
    pub fn write(&mut self, write: bool) {
        self.write = write;
    }
    pub fn append(&mut self, append: bool) {
        self.append = append;
    }
    pub fn truncate(&mut self, truncate: bool) {
        self.truncate = truncate;
    }
    pub fn create(&mut self, create: bool) {
        self.create = create;
    }
    pub fn create_new(&mut self, create_new: bool) {
        self.create_new = create_new;
    }

    /// The `TUint aFileMode` for these options, or the reason they are not a valid
    /// combination — the same three `std` rejects.
    fn mode(&self) -> io::Result<u32> {
        let writing = self.write || self.append;
        if !self.read && !writing {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "an open needs read, write or append",
            ));
        }
        if !writing && (self.truncate || self.create || self.create_new) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "truncate, create and create_new need write access",
            ));
        }
        if self.append && self.truncate {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "append and truncate are not compatible",
            ));
        }
        let access = if writing { EFILE_WRITE } else { EFILE_READ };
        Ok(access | EFILE_SHARE_ANY)
    }
}

/// An open file. The handle is closed on drop, which is what commits what the file
/// server still holds.
pub struct File {
    handle: UnsafeCell<RFile>,
}

// SAFETY: the handle is one `TInt` and every operation on it is a kernel call that the
// file server serialises. See `session.rs` for the thread-ownership caveat, which is a
// run-time `KErrBadHandle` and not a data race.
unsafe impl Send for File {}
// SAFETY: as above.
unsafe impl Sync for File {}

impl File {
    pub fn open(path: &crate::path::Path, opts: &OpenOptions) -> io::Result<File> {
        let mode = opts.mode()?;
        let path = PathBuf16::new(path.to_str().ok_or_else(non_utf8)?)?;
        let file = File { handle: UnsafeCell::new(RFile { handle: 0, sub_session_handle: 0 }) };
        let code = with_session(|session| {
            let fs = session.as_rfs();
            let this = file.handle.get();
            let name = path.as_tdesc16();
            // SAFETY: all three are non-leaving efsrv members taking `this` as argument
            // 0; the session and the descriptor are borrowed for the call only.
            Ok(unsafe {
                if opts.create_new {
                    // `RFile::Create` is the one that fails when the file is there.
                    RFile_Create(this, fs, name, mode)
                } else if opts.truncate {
                    // `RFile::Replace` creates or truncates, which is `File::create`.
                    RFile_Replace(this, fs, name, mode)
                } else {
                    let code = RFile_Open(this, fs, name, mode);
                    if code != 0 && opts.create {
                        RFile_Create(this, fs, name, mode)
                    } else {
                        code
                    }
                }
            })
        })?;
        if code != 0 {
            return Err(io::Error::from_raw_os_error(code));
        }
        if opts.append {
            file.seek(SeekFrom::End(0))?;
        }
        Ok(file)
    }

    pub fn file_attr(&self) -> io::Result<FileAttr> {
        Ok(FileAttr::of_open_file(self.size_now()?))
    }

    fn size_now(&self) -> io::Result<u64> {
        let mut size = 0i32;
        // SAFETY: `RFile::Size(TInt&)` is a non-leaving const member; `this` is the
        // live handle and `size` an owned local.
        check(unsafe { RFile_Size(self.handle.get(), &mut size) })?;
        Ok(size.max(0) as u64)
    }

    pub fn fsync(&self) -> io::Result<()> {
        self.flush()
    }

    pub fn datasync(&self) -> io::Result<()> {
        self.flush()
    }

    pub fn truncate(&self, size: u64) -> io::Result<()> {
        let size = i32::try_from(size).map_err(|_| too_big())?;
        // SAFETY: a non-leaving member taking `this` as argument 0.
        check(unsafe { RFile_SetSize(self.handle.get(), size) })
    }

    pub fn read(&self, buf: &mut [u8]) -> io::Result<usize> {
        let mut des = BytesMut::new(buf)?;
        // SAFETY: `RFile::Read(TDes8&)` is a non-leaving const member; the descriptor
        // was built by euser's own constructor over `buf`, which outlives the call, and
        // the file server never writes past `iMaxLength`.
        check(unsafe { RFile_Read(self.handle.get(), des.as_tdes8()) })?;
        Ok(des.len())
    }

    pub fn read_buf(&self, cursor: BorrowedCursor<'_, u8>) -> io::Result<()> {
        crate::io::default_read_buf(|buf| self.read(buf), cursor)
    }

    /// `RFile` has no scatter/gather read: one `TDes8` per call. The generic loop is
    /// what every other platform without `readv` uses.
    pub fn read_vectored(&self, bufs: &mut [io::IoSliceMut<'_>]) -> io::Result<usize> {
        crate::io::default_read_vectored(|b| self.read(b), bufs)
    }

    pub fn is_read_vectored(&self) -> bool {
        false
    }

    pub fn write_vectored(&self, bufs: &[io::IoSlice<'_>]) -> io::Result<usize> {
        crate::io::default_write_vectored(|b| self.write(b), bufs)
    }

    pub fn is_write_vectored(&self) -> bool {
        false
    }

    pub fn write(&self, buf: &[u8]) -> io::Result<usize> {
        let des = Bytes::new(buf)?;
        // SAFETY: `RFile::Write(const TDesC8&)` is non-leaving and takes `this` as
        // argument 0; the descriptor borrows `buf` for the call and is only read.
        // Symbian writes the whole descriptor or fails, so a short write cannot happen.
        check(unsafe { RFile_Write(self.handle.get(), des.as_tdesc8()) })?;
        Ok(buf.len())
    }

    pub fn flush(&self) -> io::Result<()> {
        // SAFETY: a non-leaving member taking `this` as argument 0.
        check(unsafe { RFile_Flush(self.handle.get()) })
    }

    pub fn seek(&self, pos: SeekFrom) -> io::Result<u64> {
        let (mode, offset) = match pos {
            SeekFrom::Start(n) => (ESEEK_START, i32::try_from(n).map_err(|_| too_big())?),
            SeekFrom::End(n) => (ESEEK_END, i32::try_from(n).map_err(|_| too_big())?),
            SeekFrom::Current(n) => (ESEEK_CURRENT, i32::try_from(n).map_err(|_| too_big())?),
        };
        let mut position = offset;
        // SAFETY: `RFile::Seek(TSeek, TInt&)` is a non-leaving const member; it writes
        // the resulting position back into the owned local.
        check(unsafe { RFile_Seek(self.handle.get(), mode, &mut position) })?;
        Ok(position.max(0) as u64)
    }

    pub fn tell(&self) -> io::Result<u64> {
        self.seek(SeekFrom::Current(0))
    }

    pub fn size(&self) -> Option<io::Result<u64>> {
        Some(self.size_now())
    }

    /// Advisory locking: `RFile::Lock` and `RFile::UnLock` exist in `f32file.h` and
    /// lock a **byte range**, which is not what `std::fs::File::lock` means — `std`
    /// locks the whole file and this SDK has observed neither call. Refused rather
    /// than approximated.
    pub fn lock(&self) -> io::Result<()> {
        unsupported()
    }

    pub fn lock_shared(&self) -> io::Result<()> {
        unsupported()
    }

    pub fn try_lock(&self) -> Result<(), TryLockError> {
        Err(TryLockError::Error(unsupported_err()))
    }

    pub fn try_lock_shared(&self) -> Result<(), TryLockError> {
        Err(TryLockError::Error(unsupported_err()))
    }

    pub fn unlock(&self) -> io::Result<()> {
        unsupported()
    }

    /// `RFile::Duplicate` takes an `RFile` of another *thread*; there is no dup of a
    /// handle within one thread, so there is nothing honest to call here.
    pub fn duplicate(&self) -> io::Result<File> {
        unsupported()
    }

    pub fn set_permissions(&self, _perm: super::attr::FilePermissions) -> io::Result<()> {
        unsupported()
    }

    pub fn set_times(&self, _times: super::attr::FileTimes) -> io::Result<()> {
        unsupported()
    }
}

impl Drop for File {
    fn drop(&mut self) {
        // SAFETY: `RFile::Close()` is non-leaving and safe on a handle that was never
        // opened; the handle is not used again.
        unsafe { RHandleBase_Close(self.handle.get().cast::<RHandleBase>()) };
    }
}

impl crate::fmt::Debug for File {
    fn fmt(&self, f: &mut crate::fmt::Formatter<'_>) -> crate::fmt::Result {
        // SAFETY: reading the one `TInt` the handle is; nothing else touches it here.
        let handle = unsafe { (*self.handle.get()).sub_session_handle };
        f.debug_struct("File").field("handle", &handle).finish()
    }
}

pub(super) fn check(code: i32) -> io::Result<()> {
    if code == 0 { Ok(()) } else { Err(io::Error::from_raw_os_error(code)) }
}

fn too_big() -> io::Error {
    io::Error::new(io::ErrorKind::FileTooLarge, "a Symbian file offset is a TInt")
}

fn non_utf8() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidFilename,
        "a Symbian path is UTF-16 and is built from a UTF-8 Rust path",
    )
}
