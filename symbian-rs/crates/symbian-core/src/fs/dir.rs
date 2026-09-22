//! `Dir`: a directory as `RFs::GetDir` reads it — one call, one `CDir`, and every entry
//! already inside it.
//!
//! # Parity with C++, and why the entries borrow
//!
//! The C++ idiom is
//!
//! ```cpp
//! CDir* dir;
//! User::LeaveIfError(fs.GetDir(path, KEntryAttMaskSupported, ESortNone, dir));
//! for (TInt i = 0; i < dir->Count(); i++) { const TEntry& e = (*dir)[i]; ... }
//! delete dir;
//! ```
//!
//! — one file-server round trip, the `CDir` the server fills, and **no allocation per
//! entry**: `(*dir)[i]` is a reference into it. This type does exactly that.
//! [`DirEntry`] is a pointer into the `CDir` plus a lifetime tying it to the [`Dir`],
//! and [`Name`] is the `TEntry`'s own UTF-16 code units.
//!
//! Measured in `examples/files` with `User::CountAllocCells` (experiment 98): holding a
//! `Dir` costs **4** heap cells — the `CDir` is a small tree, not one cell — and
//! dropping it returns all 4, on the first read as on the second. Walking every entry,
//! comparing every name and reading every size costs **0**. The Rust side of
//! [`Dir::read`] allocates nothing (the path and the pattern are stack buffers), so the
//! 4 are efsrv's own, inside `GetDir`, and a C++ caller of the same function holds the
//! same 4. That last step is derived from the code, not yet measured on the C++ side.
//!
//! `std`'s `read_dir` cannot do that: its `DirEntry::file_name` returns an owned
//! `OsString` and its `path` an owned `PathBuf`, so the `std` shape of this SDK pays two
//! heap cells per entry and a clone per call. That is `std`'s signature, not a choice
//! made here, and it is the reason the `no_std` shape is not simply a copy of it.
//!
//! # `.` and `..`
//!
//! Skipped, as every `read_dir` does. Whether this file server produces them at all is
//! not the point; filtering them is the contract.
use core::fmt;
use core::marker::PhantomData;
use core::ptr;

use symbian_sys::des::{KMASK_DES_LENGTH_16, TDesC16, TDesC16_Compare, TDesC16_Ptr};
use symbian_sys::efsrv::{
    CDir, CDir_At, CDir_Count, ESORT_NONE, KENTRY_ATT_DIR, KENTRY_ATT_MATCH_MASK,
    KENTRY_ATT_VOLUME, RFs_GetDir, TENTRY_OFFSET_ATT, TENTRY_OFFSET_NAME, TENTRY_OFFSET_SIZE,
    TEntry,
};
use symbian_sys::shim::symrs_f32_dir_delete;

use super::{FileServer, MAX_FILE_NAME, path_of};
use crate::des::{Buf16, DesC16};
use crate::error::{Result, check};

/// A directory's entries, read in one `RFs::GetDir` call and freed on drop.
pub struct Dir {
    dir: *mut CDir,
    count: i32,
}

impl Dir {
    /// Reads every entry of the directory `path` names.
    ///
    /// `f32file.h` says the path of a directory to search "should always end with a
    /// backslash character. When trailing backslash is not present then it is
    /// considered as file", so one is added when missing, followed by the `*` wildcard
    /// that matches every name. The attribute mask is `KEntryAttMatchMask`, which also
    /// matches directories, hidden and system entries — `KEntryAttNormal` would not.
    pub fn read(fs: &mut FileServer, path: &str) -> Result<Self> {
        let mut pattern = path_of(path)?;
        if !path.ends_with('\\') {
            pattern.push('\\')?;
        }
        pattern.push('*')?;
        let mut dir: *mut CDir = ptr::null_mut();
        // SAFETY: a non-leaving `const` member (efsrv traps its private `GetDirL`
        // itself) taking `this` as argument 0, a descriptor it only reads, two
        // scalars, and `&mut dir` as the `CDir*&` it writes the answer into. It uses
        // the cleanup stack inside that trap, which is why every Rust entry point
        // installs one (`symbian_sys::cleanup`, experiment 97).
        check(unsafe {
            RFs_GetDir(
                fs.as_rfs(),
                pattern.as_tdesc16(),
                KENTRY_ATT_MATCH_MASK,
                ESORT_NONE,
                &mut dir,
            )
        })?;
        // TODO: success with a null `CDir` (not observed). Refused rather than
        // treated as empty, because an empty directory is a `CDir` with no entries.
        if dir.is_null() {
            return Err(crate::SymbianError::of(crate::ErrorKind::General));
        }
        // SAFETY: `dir` is the `CDir` the server just allocated for this call;
        // `Count` is a non-virtual `const` member taking only `this` and cannot leave.
        let count = unsafe { CDir_Count(dir) };
        Ok(Self { dir, count })
    }

    /// The entries, `.` and `..` excluded.
    pub fn iter(&self) -> Iter<'_> {
        Iter { dir: self, next: 0 }
    }
}

impl Drop for Dir {
    fn drop(&mut self) {
        // SAFETY: the shim runs `delete` through `~CDir()`'s vtable slot on the pointer
        // this value owns and hands out nowhere; null-safe, and it cannot leave. Every
        // `DirEntry` borrows `self`, so none outlives this.
        unsafe { symrs_f32_dir_delete(self.dir) };
    }
}

impl<'a> IntoIterator for &'a Dir {
    type Item = DirEntry<'a>;
    type IntoIter = Iter<'a>;

    fn into_iter(self) -> Iter<'a> {
        self.iter()
    }
}

/// The cursor over a [`Dir`].
pub struct Iter<'a> {
    dir: &'a Dir,
    next: i32,
}

impl<'a> Iterator for Iter<'a> {
    type Item = DirEntry<'a>;

    fn next(&mut self) -> Option<DirEntry<'a>> {
        while self.next < self.dir.count {
            let index = self.next;
            self.next += 1;
            // SAFETY: `index` is below `Count()` — `CArrayPakFlat` panics on one that
            // is not, uncatchably — and the `CDir` lives as long as `'a`. `operator[]`
            // is a non-virtual `const` member returning a reference, a pointer under
            // the EABI.
            let entry = DirEntry {
                entry: unsafe { CDir_At(self.dir.dir, index) },
                _dir: PhantomData,
            };
            if !entry.name().is_dot() {
                return Some(entry);
            }
        }
        None
    }
}

/// One entry, read in place out of the `CDir`: nothing is copied until asked for.
#[derive(Clone, Copy)]
pub struct DirEntry<'a> {
    entry: *const TEntry,
    _dir: PhantomData<&'a Dir>,
}

impl<'a> DirEntry<'a> {
    /// The 32-bit word at `offset` in the `TEntry`, at the offsets experiment 79
    /// measured.
    fn word_at(&self, offset: usize) -> u32 {
        // SAFETY: `entry` points at a live `TEntry` inside the `CDir`, which `'a`
        // keeps alive; every offset used is 4-aligned and inside the stored element.
        unsafe {
            self.entry
                .cast::<u8>()
                .add(offset)
                .cast::<u32>()
                .read_unaligned()
        }
    }

    /// The entry's name, as the file server stored it.
    pub fn name(&self) -> Name<'a> {
        // SAFETY: `iName` is a `TBufC16` at the measured offset 28, so its address is a
        // valid `const TDesC16*`. The length is the documented low 28 bits of its
        // header word and the code units come from euser's own `TDesC16::Ptr()`, so
        // nothing assumes how a `TBufC16` lays them out. The slice borrows the `CDir`.
        let des = unsafe {
            self.entry
                .cast::<u8>()
                .add(TENTRY_OFFSET_NAME)
                .cast::<TDesC16>()
        };
        let units = unsafe {
            let len = (des.cast::<u32>().read_unaligned() & KMASK_DES_LENGTH_16) as usize;
            core::slice::from_raw_parts(TDesC16_Ptr(des), len)
        };
        Name { units, des }
    }

    /// The `KEntryAtt*` bits (`iAtt`).
    pub fn attributes(&self) -> u32 {
        self.word_at(TENTRY_OFFSET_ATT)
    }

    /// The size in bytes (`iSize`), read unsigned as `f32file.h` prescribes for files
    /// over 2 GB.
    pub fn size(&self) -> u64 {
        u64::from(self.word_at(TENTRY_OFFSET_SIZE))
    }

    /// Whether the entry is a directory (`KEntryAttDir`).
    pub fn is_dir(&self) -> bool {
        self.attributes() & KENTRY_ATT_DIR != 0
    }

    /// Whether the entry is an ordinary file: not a directory and not a volume label.
    pub fn is_file(&self) -> bool {
        self.attributes() & (KENTRY_ATT_DIR | KENTRY_ATT_VOLUME) == 0
    }
}

/// A name inside a [`Dir`], still in the file server's UTF-16.
///
/// It compares with a `&str` and prints with `{}` without allocating; [`Name::to_str`]
/// decodes it into a caller's buffer for code that needs a `&str`.
///
/// `==` is exact, as Rust's is. The file systems under it are case-insensitive, and C++
/// code often uses `CompareF`; a name that differs only in case is not equal here.
#[derive(Clone, Copy)]
pub struct Name<'a> {
    units: &'a [u16],
    des: *const TDesC16,
}

impl<'a> Name<'a> {
    /// The UTF-16 code units, exactly as `TEntry::iName` holds them.
    pub fn as_utf16(&self) -> &'a [u16] {
        self.units
    }

    /// Decodes the name into `out`. `KErrOverflow` if `out` is too small (a name is at
    /// most `KMaxFileName` = 256 code units, so 768 bytes always suffice) and
    /// `KErrArgument` for an unpaired surrogate.
    pub fn to_str<'b>(&self, out: &'b mut [u8]) -> Result<&'b str> {
        crate::des::decode_utf16_into(self.units, out)
    }

    fn is_dot(&self) -> bool {
        self.units == [0x2e] || self.units == [0x2e, 0x2e]
    }
}

impl PartialEq<str> for Name<'_> {
    /// `TDesC16::Compare` in euser, as C++ compares descriptors: `other` is encoded
    /// into a stack buffer with the same `push_str` every path already goes through,
    /// and the comparison loop is the one in ROM rather than a copy in this program.
    fn eq(&self, other: &str) -> bool {
        let mut theirs: Buf16<MAX_FILE_NAME> = Buf16::new();
        if theirs.push_str(other).is_err() {
            // Longer than any name the file server can hold, or not valid UTF-16:
            // either way it cannot be this name.
            return false;
        }
        // SAFETY: both are live descriptors — `des` is the `TEntry`'s `iName` inside the
        // `CDir` this borrows, `theirs` is on this frame — and `Compare` is a
        // non-leaving `const` member that only reads them.
        unsafe { TDesC16_Compare(self.des, theirs.as_tdesc16()) == 0 }
    }
}

impl PartialEq<&str> for Name<'_> {
    fn eq(&self, other: &&str) -> bool {
        *self == **other
    }
}

/// Prints the name, with U+FFFD for an unpaired surrogate — the same rule as
/// `String::from_utf16_lossy`, because a formatter cannot fail on its data.
impl fmt::Display for Name<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for c in char::decode_utf16(self.units.iter().copied()) {
            fmt::Write::write_char(f, c.unwrap_or(char::REPLACEMENT_CHARACTER))?;
        }
        Ok(())
    }
}
