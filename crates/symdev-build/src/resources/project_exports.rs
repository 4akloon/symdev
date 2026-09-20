//! `ProjectExports`: what `PRJ_EXPORTS` asks for, done without writing into the SDK.
use std::path::{Path, PathBuf};

use symdev_core::{Error, Result};

use crate::model::BldExport;
use crate::project::HostPath;

/// The `PRJ_EXPORTS` lines of one `bld.inf`, staged for this build.
///
/// The SDK copies each exported file into its own tree, usually into
/// `epoc32/include`. symdev will not write into the user's SDK: it copies those into the
/// project's build directory instead, which is already the first `-I` of every compile
/// and resource compile, so an exported header resolves exactly as it would have. An
/// export aimed anywhere else — a device drive, an archive to unpack at the SDK root —
/// is named in a warning rather than skipped silently (mmp-frontend-spec.md §4.3, §4.6).
pub struct ProjectExports;

impl ProjectExports {
    /// Copy what can be copied; return one sentence per export that was not.
    pub fn stage(exports: &[BldExport], bld_dir: &Path, build_dir: &Path) -> Result<Vec<String>> {
        let mut warnings = Vec::new();
        for export in exports {
            if export.zip {
                warnings.push(format!(
                    "PRJ_EXPORTS :zip {}: an archive is unpacked at the SDK root, and symdev \
                     does not write there; unpack it yourself if the build needs it",
                    export.source
                ));
                continue;
            }
            let source = HostPath::find(bld_dir, &export.source).ok_or_else(|| {
                Error::Other(format!(
                    "PRJ_EXPORTS {}: not found under {}",
                    export.source,
                    bld_dir.display()
                ))
            })?;
            let Some(under_include) = Self::under_include(export) else {
                warnings.push(format!(
                    "PRJ_EXPORTS {} {}: symdev does not write into the SDK tree, so this \
                     export was not made; copy it yourself if the build needs it",
                    export.source,
                    export.dest.clone().unwrap_or_default()
                ));
                continue;
            };
            let target = build_dir.join(&under_include);
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| Error::Other(format!("create {}: {e}", parent.display())))?;
            }
            std::fs::copy(&source, &target).map_err(|e| {
                Error::Other(format!(
                    "export {} to {}: {e}",
                    source.display(),
                    target.display()
                ))
            })?;
        }
        Ok(warnings)
    }

    /// The path under `epoc32/include` this export lands at, when it lands there at all.
    /// The default destination is that directory, so an export with no destination — the
    /// common case — always does.
    fn under_include(export: &BldExport) -> Option<PathBuf> {
        let file = || {
            Path::new(&export.source.replace('\\', "/"))
                .file_name()
                .map(PathBuf::from)
        };
        let Some(dest) = &export.dest else {
            return file();
        };
        let dest = dest.replace('\\', "/");
        // `|` forces "relative to the bld.inf", which is not the SDK's include directory.
        let rest = dest
            .strip_prefix("/epoc32/include")
            .or_else(|| dest.strip_prefix("+/include"))?;
        let rest = rest.trim_start_matches('/');
        if rest.is_empty() || dest.ends_with('/') {
            return Some(Path::new(rest).join(file()?));
        }
        Some(PathBuf::from(rest))
    }
}
