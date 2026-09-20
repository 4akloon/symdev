//! Source lookup dialect (first existing file): last SOURCEPATH/SOURCE, then
//! MMP-directory/SOURCE, then project-root/SOURCE.
use std::path::{Path, PathBuf};

use symdev_core::{Error, Result};

pub(super) fn resolve_source(
    sourcepath: Option<&str>,
    mmp_dir: &Path,
    project_root: &Path,
    source: &str,
) -> Result<PathBuf> {
    // MMP paths use `\\` (Symbian); the host uses `/`.
    let source = source.replace('\\', "/");
    let mut candidates = Vec::new();
    if let Some(sp) = sourcepath {
        let sp = PathBuf::from(sp.replace('\\', "/"));
        if sp.is_absolute() {
            candidates.push(sp.join(&source));
        } else {
            candidates.push(mmp_dir.join(sp).join(&source));
        }
    }
    candidates.push(mmp_dir.join(&source));
    candidates.push(project_root.join(&source));
    candidates
        .into_iter()
        .find(|p| p.is_file())
        .ok_or_else(|| Error::Other(format!("source not found: {source}")))
}
