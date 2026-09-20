//! `MmpBitmap` extension methods: output names, install destination, source paths
//! ([mmp-frontend-spec.md](../../../../docs/research/mmp-frontend-spec.md) §7.5).
use std::path::{Path, PathBuf};

use symdev_core::{Error, Result};

use crate::{MmpBitmap, MmpBitmapSource};

impl MmpBitmap {
    /// The `.mbm` file name: the `START BITMAP` name, without any directory part.
    pub fn output(&self) -> Result<&str> {
        self.target
            .rsplit('\\')
            .next()
            .filter(|n| !n.is_empty())
            .ok_or_else(|| Error::Other(format!("bad START BITMAP name {}", self.target)))
    }

    /// The `.mbg` the sources `#include`: the output's basename plus `.mbg`.
    pub fn header_name(&self) -> Result<String> {
        let name = self.output()?;
        let stem = name.rsplit_once('.').map_or(name, |(head, _)| head);
        Ok(format!("{stem}.mbg"))
    }

    /// `!:` install path of the compiled `.mbm`.
    pub fn install_dest(&self) -> Result<String> {
        let file = self.output()?;
        let dir = self.targetpath.as_deref().ok_or_else(|| {
            Error::Other(format!("START BITMAP {} needs TARGETPATH", self.target))
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
}

impl MmpBitmapSource {
    /// The `.bmp` file: relative to the block's `SOURCEPATH`, else the MMP directory.
    pub fn path(&self, mmp_dir: &Path) -> PathBuf {
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
