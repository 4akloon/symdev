//! Where a package may live: never inside another installed package's directory nor
//! above one. A directory is a package exactly when it holds a receipt file, whatever
//! the receipt says, so even a damaged package is never taken for free space.

use std::fs;
use std::path::{Path, PathBuf};

use super::{SdkHome, io_at, remove_file_if_exists};
use crate::{PackageId, Receipt, Result, SdkError};

impl SdkHome {
    /// Removes `id` under the lock; `false` if there is nothing at its path. The receipt
    /// goes first, so an interrupted removal leaves an unfinished package that an install
    /// replaces and a second `uninstall` finishes. A receipt-less directory is removed
    /// only while no package holds it and it holds none: a partial id
    /// (`sdk;s60-3rd-fp2`) or an id inside a package (`gcce;12.1.0;bin`) removes nothing.
    pub fn uninstall(&self, id: &PackageId) -> Result<bool> {
        let _lock = self.lock()?;
        if let Some(owner) = self.enclosing(id)? {
            return Err(SdkError::Other(format!(
                "{id} is not a package: it is a directory inside the installed package \
                 {owner}; run `symdev sdk uninstall {}` to remove that package",
                owner.shell_word()
            )));
        }
        let dir = self.package_dir(id);
        if fs::symlink_metadata(&dir).is_err() {
            return Ok(false);
        }
        if !Self::has_receipt(&dir)
            && let Some(nested) = self.nested(&dir)?
        {
            return Err(SdkError::Other(format!(
                "{id} is not a package: its directory holds the installed package {nested}; \
                 run `symdev sdk uninstall {}` to remove that package",
                nested.shell_word()
            )));
        }
        remove_file_if_exists(&dir.join(Receipt::FILE))?;
        fs::remove_dir_all(&dir).map_err(io_at(&dir))?;
        // Drop the now empty `gcce/` and the like; stop at the first non-empty one.
        let mut parent = dir.parent();
        while let Some(p) = parent
            && p != self.root
            && fs::remove_dir(p).is_ok()
        {
            parent = p.parent();
        }
        Ok(true)
    }

    /// Refuses to install `id` inside an installed package or above one; the caller
    /// holds the lock.
    pub(super) fn check_placement(&self, id: &PackageId) -> Result<()> {
        if let Some(owner) = self.enclosing(id)? {
            return Err(SdkError::Other(format!(
                "cannot install {id}: its directory would be inside the installed package \
                 {owner}; one package cannot hold another, so check the id"
            )));
        }
        let dir = self.package_dir(id);
        if !Self::has_receipt(&dir)
            && let Some(nested) = self.nested(&dir)?
        {
            return Err(SdkError::Other(format!(
                "cannot install {id}: its directory holds the installed package {nested}; \
                 one package cannot hold another, so check the id"
            )));
        }
        Ok(())
    }

    /// The installed package whose directory holds `id`'s, if any: the nearest parent
    /// directory below the home that has a receipt.
    fn enclosing(&self, id: &PackageId) -> Result<Option<PackageId>> {
        let segments: Vec<_> = id.segments().collect();
        for n in (1..segments.len()).rev() {
            let dir: PathBuf = self.root.join(segments[..n].iter().collect::<PathBuf>());
            if Self::has_receipt(&dir) {
                return Self::owner(&dir);
            }
        }
        Ok(None)
    }

    /// An installed package below `dir` (not following symlinks).
    fn nested(&self, dir: &Path) -> Result<Option<PackageId>> {
        let mut todo = vec![dir.to_path_buf()];
        while let Some(here) = todo.pop() {
            let entries = match fs::read_dir(&here) {
                Ok(entries) => entries,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
                Err(source) => return Err(io_at(&here)(source)),
            };
            for entry in entries {
                let path = entry.map_err(io_at(&here))?.path();
                let is_dir = fs::symlink_metadata(&path).is_ok_and(|m| m.is_dir());
                if !is_dir {
                    continue;
                }
                if Self::has_receipt(&path) {
                    return Self::owner(&path);
                }
                todo.push(path);
            }
        }
        Ok(None)
    }

    /// The id in the receipt of `dir`; a damaged one is an error naming the fix.
    fn owner(dir: &Path) -> Result<Option<PackageId>> {
        Ok(Receipt::read(dir)?.map(|receipt| receipt.id))
    }

    /// Whether `dir` holds a receipt file (read or not: a damaged one still marks a
    /// package).
    fn has_receipt(dir: &Path) -> bool {
        fs::symlink_metadata(dir.join(Receipt::FILE)).is_ok()
    }
}
