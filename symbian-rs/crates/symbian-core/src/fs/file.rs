//! `File`: an open file (`RFile`).
//!
//! Every member used here returns a `TInt` and none of them leaves (`f32file.h`), so
//! there is no C++ in the path. The handle is closed when the value is dropped;
//! `RFile::Close` also commits what the file server still holds for this handle.
use symbian_sys::efsrv::{
    EFILE_READ, EFILE_SHARE_ANY, EFILE_WRITE, ESEEK_CURRENT, ESEEK_END, ESEEK_START, RFile,
    RFile_Close, RFile_Create, RFile_Flush, RFile_Open, RFile_Read, RFile_ReadAt, RFile_Replace,
    RFile_Seek, RFile_SetSize, RFile_Size, RFile_Write,
};

use super::server::FileServer;
use super::{MAX_FILE_NAME, path_of};
use crate::des::{Buf16, DesC16};
use crate::des8::{DesC8, Ptr8, PtrC8};
use crate::error::{Result, check};
use crate::{ErrorKind, SymbianError};

/// `TFileMode` as the two choices an application actually makes. The share mode is
/// always `EFileShareAny`, the mode closest to POSIX: another process may hold the
/// same file open, and the file server arbitrates nothing on our behalf.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileMode {
    /// `EFileShareAny | EFileRead`.
    Read,
    /// `EFileShareAny | EFileWrite`.
    Write,
    /// `EFileShareAny | EFileRead | EFileWrite`.
    ReadWrite,
}

impl FileMode {
    /// The `TUint aFileMode` an `RFile` member takes.
    pub const fn bits(self) -> u32 {
        let access = match self {
            Self::Read => EFILE_READ,
            Self::Write | Self::ReadWrite => EFILE_WRITE,
        };
        EFILE_SHARE_ANY | access
    }
}

/// Where a seek starts from (`TSeek`).
///
/// `ESeekAddress` has no variant: it only means anything on an execute-in-place file
/// system and its result is an address, not a position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Seek {
    /// `ESeekStart`.
    Start(i32),
    /// `ESeekCurrent`.
    Current(i32),
    /// `ESeekEnd`.
    End(i32),
}

impl Seek {
    const fn parts(self) -> (i32, i32) {
        match self {
            Self::Start(n) => (ESEEK_START, n),
            Self::Current(n) => (ESEEK_CURRENT, n),
            Self::End(n) => (ESEEK_END, n),
        }
    }
}

/// An open file.
///
/// Not `Send` or `Sync`: a sub-session handle belongs to the thread that opened it.
pub struct File {
    file: RFile,
}

impl File {
    fn closed() -> Self {
        Self {
            file: RFile {
                handle: 0,
                sub_session_handle: 0,
            },
        }
    }

    /// Opens an existing file (`RFile::Open`). `KErrNotFound` if it is not there.
    pub fn open(fs: &mut FileServer, path: &str, mode: FileMode) -> Result<Self> {
        Self::opened_by(fs, &path_of(path)?, mode, RFile_Open)
    }

    /// [`File::open`] for a path already in a descriptor, as a Symbian call handed it
    /// back — `BaflUtils::NearestLanguageFile` rewrites a `TFileName` in place.
    pub(crate) fn open_des(
        fs: &mut FileServer,
        path: &Buf16<MAX_FILE_NAME>,
        mode: FileMode,
    ) -> Result<Self> {
        Self::opened_by(fs, path, mode, RFile_Open)
    }

    /// Creates a new file (`RFile::Create`). `KErrAlreadyExists` if one is already
    /// there, which is what makes it the honest `create_new`.
    pub fn create_new(fs: &mut FileServer, path: &str, mode: FileMode) -> Result<Self> {
        Self::opened_by(fs, &path_of(path)?, mode, RFile_Create)
    }

    /// Creates the file, or truncates an existing one to zero (`RFile::Replace`). The
    /// directory has to exist already.
    pub fn replace(fs: &mut FileServer, path: &str, mode: FileMode) -> Result<Self> {
        Self::opened_by(fs, &path_of(path)?, mode, RFile_Replace)
    }

    /// The three opening calls differ only in the symbol: same arguments, same result,
    /// all three non-leaving.
    fn opened_by(
        fs: &mut FileServer,
        path: &Buf16<MAX_FILE_NAME>,
        mode: FileMode,
        open: unsafe extern "C" fn(
            *mut RFile,
            *mut symbian_sys::efsrv::RFs,
            *const symbian_sys::des::TDesC16,
            u32,
        ) -> i32,
    ) -> Result<Self> {
        let mut file = Self::closed();
        // SAFETY: `this` in argument 0 per the observed member ABI, then the session
        // (borrowed mutably for the call, as `RFs&` is), the path descriptor (borrowed
        // and only read) and the mode. All three calls are non-leaving and report every
        // failure as the returned `TInt`; on failure the handle is left as it was, and
        // `Drop` is safe on a zero handle either way.
        let code = unsafe { open(&mut file.file, fs.as_rfs(), path.as_tdesc16(), mode.bits()) };
        check(code)?;
        Ok(file)
    }

    /// Reads from the current position into `buf` and returns how many bytes arrived
    /// (`RFile::Read`).
    ///
    /// A short read is **not** an error: the file server returns what it has, and zero
    /// bytes with `KErrNone` is end of file.
    pub fn read(&self, buf: &mut [u8]) -> Result<usize> {
        let mut des = Ptr8::new(buf)?;
        // SAFETY: a `const` member, so `this` is shared. The descriptor is a real
        // `TPtr8` built by euser over `buf`, with `iMaxLength` equal to `buf.len()`, so
        // every byte the server writes is inside the slice, which stays borrowed
        // mutably for the call. Non-leaving.
        let code = unsafe { RFile_Read(&self.file, des.as_tdes8()) };
        check(code)?;
        Ok(des.len())
    }

    /// Reads from byte `at` into `buf`, without moving the current position, and returns
    /// how many bytes arrived (`RFile::Read(TInt aPos, TDes8&)`). Fewer than asked for
    /// means the file ended; that is not an error.
    pub fn read_at(&self, at: u32, buf: &mut [u8]) -> Result<usize> {
        let at = i32::try_from(at).map_err(|_| SymbianError::of(ErrorKind::TooBig))?;
        let mut des = Ptr8::new(buf)?;
        // SAFETY: as `read`, with the position as a scalar argument between `this` and
        // the descriptor. Non-leaving.
        let code = unsafe { RFile_ReadAt(&self.file, at, des.as_tdes8()) };
        check(code)?;
        Ok(des.len())
    }

    /// Writes all of `buf` at the current position (`RFile::Write`).
    ///
    /// Symbian writes the whole descriptor or fails: there is no short write here, so
    /// there is nothing for a caller to loop over.
    pub fn write(&mut self, buf: &[u8]) -> Result<()> {
        let des = PtrC8::new(buf)?;
        // SAFETY: `this` in argument 0; the descriptor is a real `TPtrC8` built by
        // euser over `buf`, borrowed and only read for the call. Non-leaving.
        let code = unsafe { RFile_Write(&mut self.file, des.as_tdesc8()) };
        check(code).map(|_| ())
    }

    /// The size in bytes (`RFile::Size`).
    pub fn size(&self) -> Result<u64> {
        let mut size = 0i32;
        // SAFETY: a `const` member; `size` is a live `TInt` the call writes once.
        let code = unsafe { RFile_Size(&self.file, &mut size) };
        check(code)?;
        // `TInt` is signed and a size is not; a file above 2 GB is the case
        // `f32file.h` says to read as unsigned.
        Ok(u64::from(size as u32))
    }

    /// Truncates or extends the file (`RFile::SetSize`). It must be open for writing.
    ///
    /// `KErrTooBig` for a size a `TInt` cannot express: the file server's size is 32
    /// bits and this SDK does not pretend otherwise.
    pub fn set_size(&mut self, size: u64) -> Result<()> {
        let Ok(size) = i32::try_from(size) else {
            return Err(SymbianError::of(ErrorKind::TooBig));
        };
        // SAFETY: `this` in argument 0 and a scalar argument. Non-leaving.
        let code = unsafe { RFile_SetSize(&mut self.file, size) };
        check(code).map(|_| ())
    }

    /// Moves the current position and returns where it ended up (`RFile::Seek`).
    pub fn seek(&self, to: Seek) -> Result<u64> {
        let (mode, mut pos) = to.parts();
        // SAFETY: a `const` member; `pos` is a live `TInt` that carries the offset in
        // and the resulting absolute position out. Non-leaving.
        let code = unsafe { RFile_Seek(&self.file, mode, &mut pos) };
        check(code)?;
        Ok(u64::from(pos as u32))
    }

    /// Commits the data and the directory entry to the medium (`RFile::Flush`).
    pub fn flush(&mut self) -> Result<()> {
        // SAFETY: `this` in argument 0, no other argument. Non-leaving.
        let code = unsafe { RFile_Flush(&mut self.file) };
        check(code).map(|_| ())
    }
}

impl Drop for File {
    fn drop(&mut self) {
        // SAFETY: `RFile::Close` is a non-leaving member taking only `this`, and it is
        // safe on a handle that was never opened (both words are zero then).
        unsafe { RFile_Close(&mut self.file) }
    }
}
