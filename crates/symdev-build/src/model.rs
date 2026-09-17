use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BldInf {
    pub mmp_files: Vec<PathBuf>,
    pub test_mmp_files: Vec<PathBuf>,
    pub exports: Vec<String>,
}
