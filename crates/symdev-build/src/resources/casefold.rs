//! `SdkIncludeCaseFold`: a case-insensitive overlay for `epoc32/include`.
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use symdev_core::{Error, Result};

/// Case-insensitive view of `epoc32/include` for a case-sensitive host: the SDK was
/// written on Windows, so headers include each other with the wrong case
/// (`fbs.h` → `FbsMessage.h`, file `fbsmessage.h`). One symlink per mismatched
/// include name, pointing at the real file.
pub struct SdkIncludeCaseFold;

impl SdkIncludeCaseFold {
    const MARKER: &str = ".symdev-casefold";

    /// Build the overlay in `out` once; returns `out`.
    pub fn ensure(include: &Path, out: &Path) -> Result<PathBuf> {
        if out.join(Self::MARKER).is_file() {
            return Ok(out.to_path_buf());
        }
        let mut by_lower: BTreeMap<String, PathBuf> = BTreeMap::new();
        let mut names: BTreeSet<String> = BTreeSet::new();
        let mut stack = vec![include.to_path_buf()];
        while let Some(dir) = stack.pop() {
            let entries =
                std::fs::read_dir(&dir).map_err(|e| Error::Other(format!("read {dir:?}: {e}")))?;
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                    continue;
                }
                let Ok(rel) = path.strip_prefix(include) else {
                    continue;
                };
                let rel = rel.to_string_lossy().replace('\\', "/");
                by_lower
                    .entry(rel.to_lowercase())
                    .or_insert_with(|| path.clone());
                if let Ok(bytes) = std::fs::read(&path) {
                    include_names(&bytes, &mut names);
                }
            }
        }
        std::fs::create_dir_all(out).map_err(|e| Error::Other(format!("{out:?}: {e}")))?;
        for name in names {
            if include.join(&name).exists() {
                continue;
            }
            let Some(real) = by_lower.get(&name.to_lowercase()) else {
                continue;
            };
            let link = out.join(&name);
            if let Some(parent) = link.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| Error::Other(format!("{parent:?}: {e}")))?;
            }
            if link.symlink_metadata().is_err() {
                Self::symlink(real, &link)?;
            }
        }
        std::fs::write(out.join(Self::MARKER), include.display().to_string())
            .map_err(|e| Error::Other(format!("{out:?}: {e}")))?;
        Ok(out.to_path_buf())
    }

    #[cfg(unix)]
    fn symlink(real: &Path, link: &Path) -> Result<()> {
        std::os::unix::fs::symlink(real, link)
            .map_err(|e| Error::Other(format!("symlink {link:?}: {e}")))
    }

    #[cfg(not(unix))]
    fn symlink(_real: &Path, _link: &Path) -> Result<()> {
        Ok(())
    }
}

/// The names the `#include` lines in `bytes` ask for, with `\` normalised to `/`.
pub(super) fn include_names(bytes: &[u8], names: &mut BTreeSet<String>) {
    for line in bytes.split(|&b| b == b'\n') {
        let line = String::from_utf8_lossy(line);
        let t = line.trim_start();
        let Some(rest) = t.strip_prefix('#') else {
            continue;
        };
        let Some(rest) = rest.trim_start().strip_prefix("include") else {
            continue;
        };
        let rest = rest.trim_start();
        let close = match rest.chars().next() {
            Some('<') => '>',
            Some('"') => '"',
            _ => continue,
        };
        if let Some(end) = rest[1..].find(close) {
            let name = rest[1..1 + end].replace('\\', "/");
            if !name.is_empty() && !name.starts_with('/') && !name.contains("..") {
                names.insert(name);
            }
        }
    }
}
