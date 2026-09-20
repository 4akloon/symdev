//! `class TEntry` (`f32file.h` line 1159): what `RFs::Entry` fills in about one file or
//! directory.
//!
//! `TEntry` is the one file-API type Rust cannot simply declare field by field: its
//! copy constructor and assignment operator are `IMPORT_C`, so it is not a C type, and
//! its members run from a `TUint` through a `TTime` and a `TUidType` to a
//! `TBufC<KMaxFileName>`. Rust therefore owns **opaque storage of the measured size and
//! alignment**, lets euser's own exported default constructor build the object inside
//! it, hands `RFs::Entry` a pointer to it, and reads back only the two fields whose
//! offsets were measured.
//!
//! Measured by compiling `return sizeof(TEntry);` and `return offsetof(TEntry, iSize);`
//! with the recorded GCCE argv and reading the immediates (experiment 79):
//! `sizeof(TEntry) == 552` (`movs r0,#138; lsls r0,r0,#2`), `__alignof__(TEntry) == 8`,
//! and the offsets `iAtt 0`, `iSize 4`, `iModified 8`, `iType 16`, `iName 28`. Only the
//! first two are read here; time and UID type wait for the step that has a `SystemTime`
//! to put them in.

/// `sizeof(TEntry)`, measured.
const TENTRY_SIZE: usize = 552;

/// `offsetof(TEntry, iAtt)`, measured: the `KEntryAtt*` bits.
pub const TENTRY_OFFSET_ATT: usize = 0;

/// `offsetof(TEntry, iSize)`, measured: the size in bytes, as a `TInt`.
pub const TENTRY_OFFSET_SIZE: usize = 4;

/// `KEntryAttVolume` (`f32file.h` line 237): the entry is a volume label, not a file.
pub const KENTRY_ATT_VOLUME: u32 = 0x0008;

/// `KEntryAttDir` (`f32file.h` line 248).
pub const KENTRY_ATT_DIR: u32 = 0x0010;

/// The opaque `TEntry&` `RFs::Entry` fills; only ever seen behind a pointer.
#[repr(C)]
pub struct TEntry {
    _private: [u8; 0],
}

/// Caller-owned storage a `TEntry` is constructed into.
///
/// The alignment is the measured 8, which matters: `iModified` is a `TTime` (`TInt64`)
/// at offset 8, and a misaligned object would be a different thing from what euser
/// writes.
#[repr(C, align(8))]
pub struct TEntryStorage {
    bytes: [u8; TENTRY_SIZE],
}

impl TEntryStorage {
    /// Zeroed storage, ready for euser's constructor to build a `TEntry` into.
    pub const fn zeroed() -> Self {
        Self {
            bytes: [0; TENTRY_SIZE],
        }
    }

    /// The `TEntry&` `RFs::Entry` expects.
    pub const fn as_entry(&mut self) -> *mut TEntry {
        (self as *mut Self).cast()
    }

    /// The `TUint32` at `offset`, for the two fields whose offsets were measured.
    ///
    /// `offset` is always [`TENTRY_OFFSET_ATT`] or [`TENTRY_OFFSET_SIZE`], both of which
    /// are 4-aligned and well inside the storage, so this needs no `unsafe` and cannot
    /// read out of bounds: the bytes come out of the array by index.
    pub const fn word_at(&self, offset: usize) -> u32 {
        u32::from_ne_bytes([
            self.bytes[offset],
            self.bytes[offset + 1],
            self.bytes[offset + 2],
            self.bytes[offset + 3],
        ])
    }
}

impl Default for TEntryStorage {
    fn default() -> Self {
        Self::zeroed()
    }
}

unsafe extern "C" {
    /// `000001b8 T _ZN6TEntryC1Ev` — `TEntry::TEntry()`, efsrv.dso: the exported default
    /// constructor, called on caller-owned storage with `this` as argument 0. It is what
    /// puts the descriptor header into `iName` and the epoch into `iModified`, so that
    /// `RFs::Entry` is handed a real `TEntry` rather than 552 zero bytes.
    #[link_name = "_ZN6TEntryC1Ev"]
    pub fn TEntry_ctor(this: *mut TEntry);
}
