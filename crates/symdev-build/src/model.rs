use std::path::PathBuf;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BldInf {
    /// `PRJ_PLATFORMS`, expanded and with GCCE appended. Diagnostics only: GCCE is an
    /// optional platform the SDK adds to every component, so the list never vetoes a
    /// build (mmp-frontend-spec.md §4.2, §4.6).
    pub platforms: Vec<String>,
    pub mmp_files: Vec<PathBuf>,
    pub test_mmp_files: Vec<PathBuf>,
    pub exports: Vec<BldExport>,
    pub test_exports: Vec<BldExport>,
    /// Lines the front end parsed and did not act on, one sentence each.
    pub warnings: Vec<String>,
}

/// One `PRJ_EXPORTS` / `PRJ_TESTEXPORTS` line (§4.3).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BldExport {
    /// The file to copy, relative to the `bld.inf`'s directory.
    pub source: String,
    /// The destination as written; `None` means the section's default directory.
    pub dest: Option<String>,
    /// `:zip <archive>`: the archive is unpacked at the SDK root instead.
    pub zip: bool,
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
    /// `TARGET` inside the block: the basename the compiled resource and its `.rsg`
    /// take, instead of the `.rss` name (§6.2). Only a basename; a directory or an
    /// extension written here is discarded.
    pub target: Option<String>,
    /// `TARGETPATH` inside the block (install directory of the `.rsc`).
    pub targetpath: Option<String>,
    /// `HEADER`: also generate `<basename>.rsg` for C++ sources.
    pub header: bool,
    /// `HEADERONLY`: the `.rsg` and no compiled resource at all.
    pub headeronly: bool,
    /// `LANG` inside the block, overriding the file-level one (§6.3).
    pub lang: Vec<String>,
}
