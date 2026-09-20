//! `efsrv.dll` exports: the file server session (`RFs`), an open file (`RFile`) and a
//! directory entry (`TEntry`). Every mangled name is what `nm -D
//! epoc32/release/armv5/lib/efsrv.dso` prints and is cited next to the declaration.
//!
//! **None of it leaves, and that is checked rather than recalled.** `f32file.h` declares
//! no leaving member on `RFs`, `RFile`, `RDir` or `TEntry`: every `IMPORT_C … L(` in the
//! header belongs to `CDir`, `CDirScan`, `CFileBase`, `CFileMan` or `TOpenFileScan`, none
//! of which this SDK calls. So the whole file API is declared here and called directly,
//! with `this` as argument 0 — the member ABI observed in experiment 78 — and no C++
//! shim stands in the path (design spec §7, §11 step 71).
mod entry;
mod rfile;
mod rfs;

pub use entry::{
    KENTRY_ATT_DIR, KENTRY_ATT_VOLUME, TENTRY_OFFSET_ATT, TENTRY_OFFSET_SIZE, TEntry, TEntry_ctor,
    TEntryStorage,
};
pub use rfile::{
    EFILE_READ, EFILE_SHARE_ANY, EFILE_SHARE_EXCLUSIVE, EFILE_SHARE_READERS_ONLY,
    EFILE_SHARE_READERS_OR_WRITERS, EFILE_WRITE, ESEEK_CURRENT, ESEEK_END, ESEEK_START, RFile,
    RFile_Close, RFile_Create, RFile_Flush, RFile_Open, RFile_Read, RFile_Replace, RFile_Seek,
    RFile_SetSize, RFile_Size, RFile_Write,
};
pub use rfs::{
    KFILE_SERVER_DEFAULT_MESSAGE_SLOTS, RFs, RFs_Att, RFs_Connect, RFs_Delete, RFs_Entry,
    RFs_MkDir, RFs_MkDirAll, RFs_Rename,
};
