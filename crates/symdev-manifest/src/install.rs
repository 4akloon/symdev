//! `[[install]]`: files the package installs besides the built binary and its resources.
use std::path::PathBuf;

use serde::Deserialize;

use crate::error::{Error, Result};

/// One `[[install]]` entry: a file to carry in the package and where it lands on the
/// phone. Apps that ship a bitmap store, a font or a second icon container need this;
/// the EXE and the MMP's own resources come from the build, not from here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallFile {
    /// Source, relative to the project root. A build output (`build/games.mbm`) is fine.
    pub source: PathBuf,
    /// Install destination, normalised to `!:\…` — the drive the user picks at install.
    pub dest: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawInstall {
    pub(crate) source: PathBuf,
    pub(crate) dest: String,
}

impl InstallFile {
    pub(crate) fn validate(raw: RawInstall) -> Result<Self> {
        if raw.source.as_os_str().is_empty() {
            return Err(Error::Invalid("install.source is empty".into()));
        }
        if raw.source.is_absolute() {
            return Err(Error::Invalid(format!(
                "install.source `{}` must be relative to the project root",
                raw.source.display()
            )));
        }
        Ok(Self {
            source: raw.source,
            dest: Self::dest(&raw.dest)?,
        })
    }

    /// `\resource\apps\x.mbm` and `!:\resource\apps\x.mbm` both mean the user's drive;
    /// forward slashes are accepted for the operator's comfort. A fixed drive letter is
    /// not accepted: no package we have recorded uses one.
    pub(crate) fn dest(dest: &str) -> Result<String> {
        let dest = dest.trim().replace('/', "\\");
        let path = match dest.strip_prefix("!:") {
            Some(rest) => rest,
            None if dest.starts_with('\\') => dest.as_str(),
            None => {
                return Err(Error::Invalid(format!(
                    "install.dest `{dest}` must start with `\\` or `!:\\` (an absolute path \
                     on the drive the user installs to)"
                )));
            }
        };
        if !path.starts_with('\\') || path.ends_with('\\') || path.len() < 2 {
            return Err(Error::Invalid(format!(
                "install.dest `{dest}` must name a file, as in `\\resource\\apps\\games.mbm`"
            )));
        }
        Ok(format!("!:{path}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dest(s: &str) -> Result<String> {
        InstallFile::dest(s)
    }

    #[test]
    fn bare_absolute_path_gets_the_user_drive() {
        assert_eq!(
            dest("\\resource\\apps\\games.mbm").unwrap(),
            "!:\\resource\\apps\\games.mbm"
        );
    }

    #[test]
    fn explicit_user_drive_and_forward_slashes_are_the_same_path() {
        assert_eq!(
            dest("!:/resource/apps/games.mbm").unwrap(),
            "!:\\resource\\apps\\games.mbm"
        );
    }

    #[test]
    fn relative_directory_and_fixed_drive_are_rejected() {
        for bad in ["resource\\games.mbm", "c:\\resource\\games.mbm", "!:\\", ""] {
            assert!(dest(bad).is_err(), "{bad} must be rejected");
        }
    }

    #[test]
    fn source_must_be_relative_to_the_project() {
        let raw = RawInstall {
            source: PathBuf::from("/etc/passwd"),
            dest: "\\resource\\apps\\x.mbm".into(),
        };
        assert!(InstallFile::validate(raw).is_err());
    }
}
