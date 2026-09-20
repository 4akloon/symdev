//! `class RFile`, an open file, from `nm -D epoc32/release/armv5/lib/efsrv.dso`.
//!
//! Every member below returns a `TInt` and none of them leaves (`f32file.h`). They are
//! non-static members, called with `this` as argument 0 under the observed member ABI,
//! so no C++ shim is involved. The `const` members (`Read`, `Size`, `Seek`) are declared
//! with a `*const RFile` for the same reason a C++ caller may call them on a const
//! reference; the handle itself is not mutated.

use crate::des::TDesC16;
use crate::des8::{TDes8, TDesC8};

/// `RFile` is `RSubSessionBase` is `RHandleBase`: the session handle and the
/// sub-session handle, two words — `sizeof(RFile) == 8`, measured by compiling
/// `return sizeof(RFile);` with the recorded GCCE argv and reading the immediate
/// (experiments 78 and 79). A default-constructed `RFile` is both words zero.
#[repr(C)]
pub struct RFile {
    pub handle: i32,
    pub sub_session_handle: i32,
}

/// `TFileMode` (`f32file.h` line 838). The share mode is the low part and the access
/// mode is OR-ed into it; `EFileRead` and `EFileStream` are both `0`, so a mode of
/// `EFileShareAny` alone is a shared read.
pub const EFILE_SHARE_EXCLUSIVE: u32 = 0;
/// `EFileShareReadersOnly`.
pub const EFILE_SHARE_READERS_ONLY: u32 = 1;
/// `EFileShareAny`: the mode closest to POSIX, where another process may also have the
/// file open.
pub const EFILE_SHARE_ANY: u32 = 2;
/// `EFileShareReadersOrWriters`.
pub const EFILE_SHARE_READERS_OR_WRITERS: u32 = 3;
/// `EFileRead = 0`: reading is what a file mode grants when `EFileWrite` is absent.
pub const EFILE_READ: u32 = 0;
/// `EFileWrite = 0x200`.
pub const EFILE_WRITE: u32 = 0x200;

/// `TSeek` (`f32file.h` line 1088): `ESeekAddress` is `0` and is only meaningful on a
/// execute-in-place file system, so it has no constant here.
pub const ESEEK_START: i32 = 1;
/// `ESeekCurrent`.
pub const ESEEK_CURRENT: i32 = 2;
/// `ESeekEnd`.
pub const ESEEK_END: i32 = 3;

unsafe extern "C" {
    /// `00000170 T _ZN5RFile4OpenER3RFsRK7TDesC16j` — `RFile::Open(RFs& aFs, const
    /// TDesC16& aName, TUint aFileMode)`: opens an existing file. `KErrNotFound` if it
    /// is not there.
    #[link_name = "_ZN5RFile4OpenER3RFsRK7TDesC16j"]
    pub fn RFile_Open(
        this: *mut RFile,
        fs: *mut super::rfs::RFs,
        name: *const TDesC16,
        mode: u32,
    ) -> i32;

    /// `000001a0 T _ZN5RFile6CreateER3RFsRK7TDesC16j` — `RFile::Create(RFs&, const
    /// TDesC16&, TUint)`: creates a new file. `KErrAlreadyExists` if one is there, which
    /// is what makes it `OpenOptions::create_new`.
    #[link_name = "_ZN5RFile6CreateER3RFsRK7TDesC16j"]
    pub fn RFile_Create(
        this: *mut RFile,
        fs: *mut super::rfs::RFs,
        name: *const TDesC16,
        mode: u32,
    ) -> i32;

    /// `000001ac T _ZN5RFile7ReplaceER3RFsRK7TDesC16j` — `RFile::Replace(RFs&, const
    /// TDesC16&, TUint)`: creates the file, or truncates it to zero if it exists. The
    /// directory must already be there.
    #[link_name = "_ZN5RFile7ReplaceER3RFsRK7TDesC16j"]
    pub fn RFile_Replace(
        this: *mut RFile,
        fs: *mut super::rfs::RFs,
        name: *const TDesC16,
        mode: u32,
    ) -> i32;

    /// `000004ac T _ZN5RFile5CloseEv` — `RFile::Close()`. Safe on a handle that was
    /// never opened, and it flushes what `Write` left in the server's cache.
    #[link_name = "_ZN5RFile5CloseEv"]
    pub fn RFile_Close(this: *mut RFile);

    /// `000003f8 T _ZNK5RFile4ReadER5TDes8` — `RFile::Read(TDes8& aDes) const`: reads
    /// from the current position up to the descriptor's `iMaxLength` and sets its
    /// length. A shorter result is **not** an error and is how end of file is reported:
    /// a read that returns zero bytes with `KErrNone` is EOF.
    #[link_name = "_ZNK5RFile4ReadER5TDes8"]
    pub fn RFile_Read(this: *const RFile, des: *mut TDes8) -> i32;

    /// `00000180 T _ZN5RFile5WriteERK6TDesC8` — `RFile::Write(const TDesC8& aDes)`:
    /// writes the whole descriptor at the current position and advances it.
    #[link_name = "_ZN5RFile5WriteERK6TDesC8"]
    pub fn RFile_Write(this: *mut RFile, des: *const TDesC8) -> i32;

    /// `0000041c T _ZNK5RFile4SizeERi` — `RFile::Size(TInt& aSize) const`.
    #[link_name = "_ZNK5RFile4SizeERi"]
    pub fn RFile_Size(this: *const RFile, size: *mut i32) -> i32;

    /// `000001b0 T _ZN5RFile7SetSizeEi` — `RFile::SetSize(TInt aSize)`: truncates or
    /// extends. The file must be open for writing.
    #[link_name = "_ZN5RFile7SetSizeEi"]
    pub fn RFile_SetSize(this: *mut RFile, size: i32) -> i32;

    /// `00000418 T _ZNK5RFile4SeekE5TSeekRi` — `RFile::Seek(TSeek aMode, TInt& aPos)
    /// const`: `aPos` is the offset on the way in and the resulting absolute position on
    /// the way out.
    #[link_name = "_ZNK5RFile4SeekE5TSeekRi"]
    pub fn RFile_Seek(this: *const RFile, mode: i32, pos: *mut i32) -> i32;

    /// `0000017c T _ZN5RFile5FlushEv` — `RFile::Flush()`: commits the data and the
    /// directory entry to the medium.
    #[link_name = "_ZN5RFile5FlushEv"]
    pub fn RFile_Flush(this: *mut RFile) -> i32;
}
