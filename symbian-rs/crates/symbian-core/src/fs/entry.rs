//! `Entry`: what `RFs::Entry` knows about one file or directory (`TEntry`).
use symbian_sys::efsrv::{
    KENTRY_ATT_DIR, KENTRY_ATT_VOLUME, TENTRY_OFFSET_ATT, TENTRY_OFFSET_SIZE, TEntry, TEntry_ctor,
    TEntryStorage,
};

use super::request::Request;
use super::session::request;
use crate::error::Result;

/// One directory entry: the attribute bits and the size.
///
/// The underlying `TEntry` also carries the modification time and the UID type. Neither
/// is exposed yet, because there is no `SystemTime` in this SDK to put a `TTime` in and
/// a `TUidType` means nothing without the application-UID machinery — inventing either
/// shape now would be a guess. The rest of the object is there and untouched.
pub struct Entry {
    storage: TEntryStorage,
}

impl Entry {
    /// A `TEntry` built by euser's own default constructor, ready to be filled.
    pub(crate) fn new() -> Self {
        let mut storage = TEntryStorage::zeroed();
        // SAFETY: `storage` is zeroed storage of the measured `sizeof(TEntry) == 552`
        // with the measured alignment of 8, so it is a valid object for the exported
        // default constructor to build into, with `this` as argument 0 under the
        // observed member ABI. `TEntry()` is declared `IMPORT_C TEntry();` in
        // `f32file.h` with no leave, and the class holds no heap cell — `iName` is a
        // `TBufC` inside the object — so nothing is allocated and nothing must be freed.
        unsafe { TEntry_ctor(storage.as_entry()) };
        Self { storage }
    }

    /// What the file server knows about the entry `path` names (`RFs::Entry`), asked on
    /// the process's session.
    pub fn of(path: &str) -> Result<Self> {
        let mut entry = Self::new();
        // SAFETY: the `TEntry` was built by its constructor just above and is borrowed
        // mutably for the call, which fills it and keeps nothing.
        unsafe { request(path, Request::Entry(entry.as_tentry())) }?;
        Ok(entry)
    }

    /// The `TEntry&` `RFs::Entry` fills.
    pub(crate) fn as_tentry(&mut self) -> *mut TEntry {
        self.storage.as_entry()
    }

    /// The `KEntryAtt*` bits (`iAtt`).
    pub fn attributes(&self) -> u32 {
        self.storage.word_at(TENTRY_OFFSET_ATT)
    }

    /// The size in bytes (`iSize`).
    ///
    /// `iSize` is a `TInt`, and `f32file.h` says of it: for a file larger than 2 GB it
    /// "must be cast to TUint in order to avoid looking like negative signed". So the
    /// word is read unsigned, which is what the header prescribes.
    pub fn size(&self) -> u64 {
        u64::from(self.storage.word_at(TENTRY_OFFSET_SIZE))
    }

    /// Whether the entry is a directory (`KEntryAttDir`).
    pub fn is_dir(&self) -> bool {
        self.attributes() & KENTRY_ATT_DIR != 0
    }

    /// Whether the entry is a volume label (`KEntryAttVolume`), which is neither a file
    /// nor a directory.
    pub fn is_volume(&self) -> bool {
        self.attributes() & KENTRY_ATT_VOLUME != 0
    }

    /// Whether the entry is an ordinary file: not a directory and not a volume label.
    pub fn is_file(&self) -> bool {
        !self.is_dir() && !self.is_volume()
    }
}
