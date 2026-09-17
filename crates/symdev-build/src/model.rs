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
    pub sourcepath: Vec<String>,
    pub systeminclude: Vec<String>,
    pub userinclude: Vec<String>,
    pub library: Vec<String>,
    pub staticlibrary: Vec<String>,
    pub capability: Vec<String>,
    pub epocstacksize: Option<String>,
    pub epocheapsize: Option<String>,
    pub epocallowdlldata: bool,
    pub resource: Vec<Vec<String>>,
}
