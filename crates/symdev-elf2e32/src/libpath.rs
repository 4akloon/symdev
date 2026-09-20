//! `LibPath`: the `--libpath` search list, as a case-insensitive filesystem would see it.
use std::path::{Path, PathBuf};

use symdev_core::{Error, Result};

/// `--libpath`: a `;`-separated list of directories holding import libraries
/// (`elf2e32 --help`); the first hit wins.
///
/// A DSO records the link name of the library it stands for, and that name need not
/// match the file on disk in case: `aknicon.dso` links as `AknIcon{000a0000}.dso`, and
/// the SDK ships the file as `aknicon{000a0000}.dso`. Windows finds it anyway. So an
/// exact name wins here, and a name that differs only in case is accepted after it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LibPath {
    dirs: Vec<PathBuf>,
}

impl LibPath {
    pub fn parse(list: &str) -> Self {
        Self {
            dirs: list
                .split(';')
                .filter(|d| !d.is_empty())
                .map(PathBuf::from)
                .collect(),
        }
    }

    pub fn find(&self, dso: &str) -> Result<PathBuf> {
        for dir in &self.dirs {
            let exact = dir.join(dso);
            if exact.is_file() {
                return Ok(exact);
            }
        }
        for dir in &self.dirs {
            if let Some(found) = Self::same_name_other_case(dir, dso) {
                return Ok(found);
            }
        }
        Err(Error::Other(format!(
            "{dso} not found in --libpath {}",
            self.dirs
                .iter()
                .map(|d| d.display().to_string())
                .collect::<Vec<_>>()
                .join(";")
        )))
    }

    fn same_name_other_case(dir: &Path, dso: &str) -> Option<PathBuf> {
        let entries = std::fs::read_dir(dir).ok()?;
        for entry in entries.flatten() {
            let path = entry.path();
            let matches = path
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.eq_ignore_ascii_case(dso));
            if matches && path.is_file() {
                return Some(path);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_name_wins_over_a_case_variant() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("euser.dso"), b"a").unwrap();
        std::fs::write(dir.path().join("EUSER.dso"), b"b").unwrap();
        let path = LibPath::parse(&dir.path().display().to_string())
            .find("euser.dso")
            .unwrap();
        assert_eq!(std::fs::read(path).unwrap(), b"a");
    }

    #[test]
    fn a_dso_named_as_the_sdk_ships_it_is_found_by_its_link_name() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("aknicon{000a0000}.dso"), b"a").unwrap();
        let found = LibPath::parse(&dir.path().display().to_string())
            .find("AknIcon{000a0000}.dso")
            .unwrap();
        assert_eq!(
            found.file_name().and_then(|n| n.to_str()),
            Some("aknicon{000a0000}.dso")
        );
    }

    #[test]
    fn an_earlier_directory_wins_and_a_miss_names_the_search_list() {
        let first = tempfile::tempdir().unwrap();
        let second = tempfile::tempdir().unwrap();
        std::fs::write(first.path().join("euser.dso"), b"1").unwrap();
        std::fs::write(second.path().join("euser.dso"), b"2").unwrap();
        let list = format!("{};{}", first.path().display(), second.path().display());
        let libpath = LibPath::parse(&list);
        assert_eq!(
            std::fs::read(libpath.find("euser.dso").unwrap()).unwrap(),
            b"1"
        );
        let err = libpath.find("nothing.dso").unwrap_err().to_string();
        assert!(err.contains("nothing.dso"), "{err}");
        assert!(err.contains(&list), "{err}");
    }
}
