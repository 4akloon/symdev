//! `MmpResource` extension methods: resource stem, install destination, source path.
use std::path::{Path, PathBuf};

use symdev_core::{Error, Result};

use crate::MmpResource;

impl MmpResource {
    /// The single language this resource is built for: its own `LANG`, else the default
    /// code `SC` (§6.3). One `.rss` per language is not implemented yet.
    pub fn language(&self) -> &str {
        self.lang.first().map(String::as_str).unwrap_or("SC")
    }

    pub fn stem(&self) -> Result<&str> {
        Path::new(&self.file)
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| Error::Other(format!("bad resource name {}", self.file)))
    }

    /// `!:` install path of the compiled `.rsc`. Registration resources go to the import
    /// directory the SDK example `.pkg` uses; others to their `TARGETPATH`.
    pub fn install_dest(&self) -> Result<String> {
        let stem = self.stem()?;
        if stem.to_ascii_lowercase().ends_with("_reg") {
            return Ok(format!("!:\\private\\10003a3f\\import\\apps\\{stem}.rsc"));
        }
        let dir = self.targetpath.as_deref().ok_or_else(|| {
            Error::Other(format!("START RESOURCE {} needs TARGETPATH", self.file))
        })?;
        let dir = dir.replace('/', "\\");
        let dir = dir.trim_end_matches('\\');
        let dir = if dir.starts_with('\\') {
            dir.to_string()
        } else {
            format!("\\{dir}")
        };
        Ok(format!("!:{dir}\\{stem}.rsc"))
    }

    /// The `.rss` file: relative to the block's `SOURCEPATH`, else the MMP directory.
    pub fn source(&self, mmp_dir: &Path) -> PathBuf {
        let rel = |p: &str| p.replace('\\', "/");
        match &self.sourcepath {
            Some(sp) if Path::new(&rel(sp)).is_absolute() => {
                Path::new(&rel(sp)).join(rel(&self.file))
            }
            Some(sp) => mmp_dir.join(rel(sp)).join(rel(&self.file)),
            None => mmp_dir.join(rel(&self.file)),
        }
    }
}
