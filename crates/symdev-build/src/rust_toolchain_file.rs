//! `RustToolchainFile`: the `rust-toolchain.toml` that tells rustup which nightly builds a
//! Rust project, and the one the Rust SDK was written for.
use std::path::{Path, PathBuf};

use symdev_core::{Error, Result};

/// A `rust-toolchain.toml` and the `[toolchain] channel` it names, if any.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RustToolchainFile {
    path: PathBuf,
    channel: Option<String>,
}

impl RustToolchainFile {
    /// The file name rustup looks for, and the one `symdev new` writes.
    pub const NAME: &'static str = "rust-toolchain.toml";

    /// The file in `dir`; `None` when there is none.
    pub fn read(dir: &Path) -> Result<Option<RustToolchainFile>> {
        let path = dir.join(Self::NAME);
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(Error::Other(format!("{}: {e}", path.display()))),
        };
        let table: toml::Table = text
            .parse()
            .map_err(|e| Error::Other(format!("{}: {e}", path.display())))?;
        let channel = table
            .get("toolchain")
            .and_then(|t| t.get("channel"))
            .and_then(|c| c.as_str())
            .map(str::to_string);
        Ok(Some(RustToolchainFile { path, channel }))
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The `[toolchain] channel`, e.g. `nightly-2026-09-19`.
    pub fn channel(&self) -> Option<&str> {
        self.channel.as_deref()
    }

    /// Fails unless this (a project's) file names the channel of `sdk`'s.
    ///
    /// The project's nightly builds the SDK's crates and its `core`; the SDK is written
    /// and tested against its own, and `-Z` flags, the target spec's fields and the
    /// patched-`std` overlay all move between nightlies.
    pub fn check_against(&self, sdk: &RustToolchainFile) -> Result<()> {
        let wanted = sdk.channel.as_deref().unwrap_or("");
        let named = match self.channel.as_deref() {
            Some(channel) if channel == wanted => return Ok(()),
            Some(channel) => format!("names the toolchain `{channel}`"),
            None => "names no toolchain channel".to_string(),
        };
        Err(Error::Other(format!(
            "{} {named}, but the Rust SDK this build resolved is built with `{wanted}` ({}); \
             set `channel = \"{wanted}\"` under [toolchain] there, or copy the SDK's file \
             over it",
            self.path.display(),
            sdk.path.display()
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file_in(dir: &Path, text: &str) -> RustToolchainFile {
        std::fs::write(dir.join(RustToolchainFile::NAME), text).unwrap();
        RustToolchainFile::read(dir).unwrap().unwrap()
    }

    const SDK: &str =
        "[toolchain]\nchannel = \"nightly-2026-09-19\"\ncomponents = [\"rust-src\"]\n";

    #[test]
    fn reads_the_channel() {
        let tmp = tempfile::tempdir().unwrap();
        assert_eq!(
            file_in(tmp.path(), SDK).channel(),
            Some("nightly-2026-09-19")
        );
    }

    #[test]
    fn a_directory_without_the_file_has_none() {
        let tmp = tempfile::tempdir().unwrap();
        assert_eq!(RustToolchainFile::read(tmp.path()).unwrap(), None);
    }

    #[test]
    fn a_file_that_is_not_toml_is_named() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("rust-toolchain.toml"), "[toolchain\n").unwrap();
        let err = RustToolchainFile::read(tmp.path()).unwrap_err().to_string();
        let path = tmp.path().join("rust-toolchain.toml");
        assert!(err.starts_with(&format!("{}: ", path.display())), "{err}");
    }

    #[test]
    fn the_same_channel_agrees() {
        let (a, b) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
        let project = file_in(a.path(), "[toolchain]\nchannel = \"nightly-2026-09-19\"\n");
        project.check_against(&file_in(b.path(), SDK)).unwrap();
    }

    #[test]
    fn another_channel_is_refused_with_the_line_to_write() {
        let (a, b) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
        let project = file_in(a.path(), "[toolchain]\nchannel = \"nightly-2026-03-01\"\n");
        let sdk = file_in(b.path(), SDK);
        let err = project.check_against(&sdk).unwrap_err().to_string();
        assert_eq!(
            err,
            format!(
                "{} names the toolchain `nightly-2026-03-01`, but the Rust SDK this build \
                 resolved is built with `nightly-2026-09-19` ({}); set `channel = \
                 \"nightly-2026-09-19\"` under [toolchain] there, or copy the SDK's file over it",
                a.path().join("rust-toolchain.toml").display(),
                b.path().join("rust-toolchain.toml").display()
            )
        );
    }

    #[test]
    fn a_file_without_a_channel_is_refused_with_the_line_to_write() {
        let (a, b) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
        let project = file_in(a.path(), "[toolchain]\ncomponents = [\"rust-src\"]\n");
        let err = project
            .check_against(&file_in(b.path(), SDK))
            .unwrap_err()
            .to_string();
        assert!(err.contains("names no toolchain channel"), "{err}");
        assert!(err.contains("`channel = \"nightly-2026-09-19\"`"), "{err}");
    }
}
