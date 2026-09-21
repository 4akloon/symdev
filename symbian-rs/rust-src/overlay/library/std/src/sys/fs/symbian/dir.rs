//! `std::fs::read_dir` over `RFs::GetDir`.
//!
//! The file server reads a whole directory into one `CDir` — it has no
//! `opendir`/`readdir` pair — so [`ReadDir`] is a cursor over an array that already
//! exists, and `next` cannot fail with anything the server would have said later.
//! Every entry therefore comes back as `Ok`, except a name that is not valid UTF-16.
//!
//! # Why `.` and `..` are dropped here
//!
//! `std::fs::read_dir` documents that its iterator excludes them. Whether this file
//! server ever produces them is not the point: filtering them is `std`'s contract, so
//! it happens on this side of the boundary rather than being left to the volume's
//! file system to decide.
//!
//! # What is eager, and why
//!
//! `DirEntry` keeps the attribute word and the size it read out of the `TEntry` while
//! the `CDir` was still alive, so `metadata()` and `file_type()` need no second call to
//! the server and cannot disagree with the listing. That is the same answer POSIX's
//! `d_type` gives and the opposite of `stat`-per-entry; on a phone the round trip is
//! what costs.

use super::attr::{FileAttr, FileType};
use super::native;
use super::session::with_session;
use crate::ffi::OsString;
use crate::fmt;
use crate::io;
use crate::path::{Path, PathBuf};
use crate::ptr;
use crate::string::String;
use symbian_sys::des::{KMASK_DES_LENGTH_16, TDesC16, TDesC16_Ptr};
use symbian_sys::efsrv::{
    CDir, CDir_At, CDir_Count, ESORT_NONE, KENTRY_ATT_MATCH_MASK, RFs_GetDir, TENTRY_OFFSET_ATT,
    TENTRY_OFFSET_NAME, TENTRY_OFFSET_SIZE, TEntry,
};
use symbian_sys::shim::symrs_f32_dir_delete;

/// The directory the file server read, and where the cursor is in it.
pub struct ReadDir {
    /// The directory that was listed; every entry's path is this joined with its name.
    root: PathBuf,
    /// The `CDir` this owns; destroyed by [`Drop`].
    dir: *mut CDir,
    count: i32,
    next: i32,
}

impl ReadDir {
    /// `RFs::GetDir` on `path`, which must name a directory.
    pub(super) fn open(path: &Path) -> io::Result<Self> {
        // `f32file.h`: "when specifying the path of a directory to search, the path
        // should always end with a backslash character. When trailing backslash is not
        // present then it is considered as file." `join` supplies that separator, and
        // the `*` after it is the file-name wildcard that matches every entry.
        let pattern = native(&path.join("*"))?;
        let mut dir: *mut CDir = ptr::null_mut();
        with_session(|session| {
            // SAFETY: a non-leaving `const` member taking `this` as argument 0, then a
            // borrowed descriptor it only reads, two scalars, and `&mut dir` as the
            // `CDir*&` it writes the answer into. On failure it writes nothing and
            // `dir` stays null.
            super::check(unsafe {
                RFs_GetDir(
                    session.as_rfs(),
                    pattern.as_tdesc16(),
                    KENTRY_ATT_MATCH_MASK,
                    ESORT_NONE,
                    &mut dir,
                )
            })
        })?;
        if dir.is_null() {
            return Err(io::Error::new(
                io::ErrorKind::Uncategorized,
                "RFs::GetDir reported success but allocated no CDir",
            ));
        }
        // SAFETY: `dir` is the `CDir` the file server just allocated for this call;
        // `Count` is a non-virtual `const` member taking only `this` and cannot leave.
        let count = unsafe { CDir_Count(dir) };
        Ok(ReadDir { root: path.to_path_buf(), dir, count, next: 0 })
    }
}

impl Drop for ReadDir {
    fn drop(&mut self) {
        // SAFETY: the shim runs `delete` through `~CDir()`'s vtable slot on a pointer
        // this value owns and hands out nowhere; it is null-safe and cannot leave.
        unsafe { symrs_f32_dir_delete(self.dir) };
    }
}

impl fmt::Debug for ReadDir {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ReadDir").field("root", &self.root).field("count", &self.count).finish()
    }
}

impl Iterator for ReadDir {
    type Item = io::Result<DirEntry>;

    fn next(&mut self) -> Option<io::Result<DirEntry>> {
        while self.next < self.count {
            let index = self.next;
            self.next += 1;
            // SAFETY: `index` is below `Count()`, which is what `CArrayPakFlat` panics
            // about and no `TRAP` could catch; the `CDir` is alive for as long as this
            // value is, and `operator[]` is a non-virtual `const` member returning a
            // reference, which is a pointer under the EABI.
            let entry = unsafe { CDir_At(self.dir, index) };
            match DirEntry::read(&self.root, entry) {
                // `std::fs::read_dir` excludes these two, on every platform.
                Ok(entry) if entry.is_dot() => continue,
                other => return Some(other),
            }
        }
        None
    }
}

/// One entry, read out of the `CDir` while it was still there.
pub struct DirEntry {
    path: PathBuf,
    name: OsString,
    attr: FileAttr,
}

impl DirEntry {
    /// Reads the three things `std` asks a `DirEntry` for out of one `TEntry`.
    fn read(root: &Path, entry: *const TEntry) -> io::Result<Self> {
        // SAFETY: `entry` points at a live `TEntry` inside the `CDir`. The two offsets
        // are the ones experiment 79 measured, both 4-aligned and inside the stored
        // element, and they are read as words rather than through a `&TEntry` Rust has
        // no layout for.
        let (att, size) = unsafe {
            let base = entry.cast::<u8>();
            (
                base.add(TENTRY_OFFSET_ATT).cast::<u32>().read_unaligned(),
                base.add(TENTRY_OFFSET_SIZE).cast::<u32>().read_unaligned(),
            )
        };
        // SAFETY: `iName` is a `TBufC16` at the measured offset 28, so its address is a
        // valid `const TDesC16*`. Its length comes from the documented low 28 bits of
        // the header word and its code units from euser's own `TDesC16::Ptr()`, so
        // nothing here assumes how a `TBufC16` stores them. `Ptr` is a non-leaving
        // `const` member taking only `this`.
        let name = unsafe {
            let des = entry.cast::<u8>().add(TENTRY_OFFSET_NAME).cast::<TDesC16>();
            let len = (des.cast::<u32>().read_unaligned() & KMASK_DES_LENGTH_16) as usize;
            crate::slice::from_raw_parts(TDesC16_Ptr(des), len)
        };
        let name = String::from_utf16(name).map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "a directory entry's name is not valid UTF-16 and has no UTF-8 spelling",
            )
        })?;
        Ok(DirEntry {
            path: root.join(&name),
            name: OsString::from(name),
            attr: FileAttr::of_dir_entry(att, u64::from(size)),
        })
    }

    /// `.` and `..`, which `std::fs::read_dir` never yields.
    fn is_dot(&self) -> bool {
        self.name == "." || self.name == ".."
    }

    pub fn path(&self) -> PathBuf {
        self.path.clone()
    }

    pub fn file_name(&self) -> OsString {
        self.name.clone()
    }

    pub fn metadata(&self) -> io::Result<FileAttr> {
        Ok(self.attr.clone())
    }

    pub fn file_type(&self) -> io::Result<FileType> {
        Ok(self.attr.file_type())
    }
}
