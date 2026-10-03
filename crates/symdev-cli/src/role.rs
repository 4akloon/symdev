//! `Role`: one binary, three programs, told apart by the name it is started under
//! (design spec §4: `install.sh` and `symdev setup-linker` make the links).
use std::ffi::OsStr;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Role {
    Cli,
    Linker,
    Rustc,
}

impl Role {
    pub fn of(argv0: &OsStr) -> Self {
        match Path::new(argv0).file_stem().and_then(OsStr::to_str) {
            Some("symdev-ld") => Self::Linker,
            Some("symdev-rustc") => Self::Rustc,
            _ => Self::Cli,
        }
    }
}
