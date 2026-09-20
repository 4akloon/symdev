use std::path::PathBuf;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BldInf {
    pub mmp_files: Vec<PathBuf>,
    pub test_mmp_files: Vec<PathBuf>,
    pub exports: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
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
    /// `MACRO` arguments, in source order: `-D<text>` for the compiler and for the
    /// resource compiler (§5.3, §6.4, §8.2).
    pub macros: Vec<String>,
    /// `OPTION <compiler> <text…>`; a repeat of the same key appends (§8.5).
    pub options: Vec<MmpOption>,
    /// `SECUREID`; the E32 image defaults it to UID3 (§10.2).
    pub secureid: Option<u32>,
    /// `VENDORID`, only ever zero here: symdev's post-linker has no `--vid` (§10.2).
    pub vendorid: Option<u32>,
    /// `LANG`: the language codes resources without their own `LANG` are built for
    /// (§6.3). Empty means the single default code `SC`.
    pub lang: Vec<String>,
    pub epocallowdlldata: bool,
    /// `DEFFILE`: frozen exports, relative to the MMP (`mmp.pm`).
    pub deffile: Option<String>,
    /// `NOSTRICTDEF`: no `u` suffix on the frozen `.def` name.
    pub nostrictdef: bool,
    pub resource: Vec<MmpResource>,
    /// Directives the front end parsed and nothing on this path reads (§5.5), one
    /// sentence each, for the caller to print.
    pub warnings: Vec<String>,
}

/// `OPTION <compiler> <text…>`: extra flags for one compiler key (§8.5). The key that
/// matters on this path is the platform name, `GCCE`; any other is inert, which is how
/// portable `.mmp` files carry `OPTION ARMCC …` as well.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MmpOption {
    pub compiler: String,
    pub text: String,
}

/// A `START RESOURCE <file> … END` block.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
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
