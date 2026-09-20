//! `MmpResource` extension methods: output names, install destination, source path
//! ([mmp-frontend-spec.md](../../../../docs/research/mmp-frontend-spec.md) §6.3).
use std::path::{Path, PathBuf};

use symdev_core::{Error, Result};

use crate::MmpResource;

impl MmpResource {
    /// The basename every output of this block takes: the block's `TARGET` if it has
    /// one — directory and extension discarded — else the `.rss` file's own stem.
    pub fn stem(&self) -> Result<String> {
        if let Some(target) = &self.target {
            let name = target.rsplit(['\\', '/']).next().unwrap_or(target);
            let name = name.rsplit_once('.').map_or(name, |(head, _)| head);
            if name.is_empty() {
                return Err(Error::Other(format!(
                    "START RESOURCE {}: empty TARGET",
                    self.file
                )));
            }
            return Ok(name.to_string());
        }
        Path::new(&self.file)
            .file_stem()
            .and_then(|s| s.to_str())
            .map(str::to_string)
            .ok_or_else(|| Error::Other(format!("bad resource name {}", self.file)))
    }

    /// The languages this block is compiled for: its own `LANG`, else the file-level
    /// one, else the single default code `SC`.
    pub fn languages(&self, file_lang: &[String]) -> Vec<String> {
        let codes: &[String] = if self.lang.is_empty() {
            file_lang
        } else {
            &self.lang
        };
        if codes.is_empty() {
            return vec!["SC".to_string()];
        }
        codes.to_vec()
    }

    /// `<basename>` + the lower case of `.R` and the language code: `SC` gives `.rsc`,
    /// `01` gives `.r01`.
    pub fn output(&self, language: &str) -> Result<String> {
        Ok(format!(
            "{}{}",
            self.stem()?,
            format!(".R{language}").to_ascii_lowercase()
        ))
    }

    /// The `.rsg` the sources `#include`: one file, whatever the language list is.
    pub fn header_name(&self) -> Result<String> {
        Ok(format!("{}.rsg", self.stem()?))
    }

    /// `!:` install path of the compiled resource. Registration resources go to the
    /// import directory the SDK example `.pkg` uses; others to their `TARGETPATH`.
    pub fn install_dest(&self, language: &str) -> Result<String> {
        let file = self.output(language)?;
        if self.stem()?.to_ascii_lowercase().ends_with("_reg") {
            return Ok(format!("!:\\private\\10003a3f\\import\\apps\\{file}"));
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
        Ok(format!("!:{dir}\\{file}"))
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
