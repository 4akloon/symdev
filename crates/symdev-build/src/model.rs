use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BldInf {
    pub mmp_files: Vec<PathBuf>,
    pub test_mmp_files: Vec<PathBuf>,
    pub exports: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mmp {
    pub target: String,
    pub target_type: String,
    pub uid: Vec<u32>,
    pub targetpath: Option<String>,
    pub source: Vec<String>,
    /// `SOURCEPATH` in effect for each `source` entry (same index).
    pub source_sourcepath: Vec<Option<String>>,
    pub sourcepath: Vec<String>,
    pub systeminclude: Vec<String>,
    pub userinclude: Vec<String>,
    pub library: Vec<String>,
    pub staticlibrary: Vec<String>,
    pub capability: Vec<String>,
    pub epocstacksize: Option<String>,
    pub epocheapsize: Option<String>,
    pub epocallowdlldata: bool,
    pub resource: Vec<MmpResource>,
}

/// A `START RESOURCE <file> … END` block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MmpResource {
    /// The `.rss` named after `START RESOURCE`.
    pub file: String,
    /// `SOURCEPATH` in effect where the block starts (the `.rss` is relative to it).
    pub sourcepath: Option<String>,
    /// `TARGETPATH` inside the block (install directory of the `.rsc`).
    pub targetpath: Option<String>,
    /// `HEADER`: also generate `<stem>.rsg` for C++ sources.
    pub header: bool,
    pub lang: Vec<String>,
}
