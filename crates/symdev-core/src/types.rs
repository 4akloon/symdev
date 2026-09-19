use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathStyle {
    Posix,
    Windows,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemotePath(String);

impl RemotePath {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }
}

impl std::fmt::Display for RemotePath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Output {
    pub status: i32,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Project {
    pub root: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Artifact {
    pub path: PathBuf,
    /// Install destination (`!:\\...`) for packaged non-EXE files; `None` for the EXE.
    pub dest: Option<String>,
}

impl Artifact {
    pub fn exe(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            dest: None,
        }
    }

    pub fn installed(path: impl Into<PathBuf>, dest: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            dest: Some(dest.into()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Package {
    pub primary: PathBuf,
    pub companions: Vec<PathBuf>,
}
