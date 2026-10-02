use std::fs::{self, File, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::{ArchiveEntry, Fetch, PackageId, Receipt, Result, SdkError, SourceSpec, TarGz};
use download_cache::DownloadCache;

/// How deep `list` looks for receipts: ids of up to this many segments.
const MAX_SEGMENTS: usize = 8;

/// Makes staging names unique within one process.
static STAGED: AtomicU64 = AtomicU64::new(0);

/// The installed packages under `SYMDEV_HOME` (`root`), and the download cache. A package
/// lives at `root/<id path>` and counts as installed only once its receipt is written.
pub struct SdkHome {
    root: PathBuf,
    cache: DownloadCache,
}

impl SdkHome {
    /// `cache` may be shared with other homes (one `XDG_CACHE_HOME`, several
    /// `SYMDEV_HOME`s): it has a lock of its own.
    pub fn new(root: PathBuf, cache: PathBuf) -> Self {
        SdkHome {
            root,
            cache: DownloadCache::new(cache),
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn package_dir(&self, id: &PackageId) -> PathBuf {
        self.root.join(id.relative_path())
    }

    /// The receipt of `id`, or `None` if it is not installed (no receipt).
    pub fn installed(&self, id: &PackageId) -> Result<Option<Receipt>> {
        let dir = self.package_dir(id);
        match Receipt::read(&dir)? {
            Some(r) if &r.id != id => Err(SdkError::Other(format!(
                "{}: the receipt names {} instead of {id}; run `symdev sdk uninstall {word} && \
                 symdev sdk install {word}`",
                dir.display(),
                r.id,
                word = id.shell_word()
            ))),
            receipt => Ok(receipt),
        }
    }

    /// Every installed package, sorted by id.
    pub fn list(&self) -> Result<Vec<Receipt>> {
        let mut found = Vec::new();
        self.collect(&self.root, 1, &mut found)?;
        found.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(found)
    }

    fn collect(&self, dir: &Path, depth: usize, found: &mut Vec<Receipt>) -> Result<()> {
        let entries = match fs::read_dir(dir) {
            Ok(entries) => entries,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(()),
            Err(source) => return Err(io_at(dir)(source)),
        };
        for entry in entries {
            let entry = entry.map_err(io_at(dir))?;
            let is_dir = entry.file_type().map_err(io_at(dir))?.is_dir();
            if !is_dir || entry.file_name().to_string_lossy().starts_with('.') {
                continue;
            }
            match Receipt::read(&entry.path())? {
                Some(receipt) => found.push(receipt),
                None if depth < MAX_SEGMENTS => self.collect(&entry.path(), depth + 1, found)?,
                None => {}
            }
        }
        Ok(())
    }

    /// Spec §3 steps 1–6. Takes `root/.lock`, re-checks `installed` after locking (another
    /// process may have finished it), refuses a place inside or above another package,
    /// clears stale staging, reuses a cached archive only if its size and SHA-256 match,
    /// deletes a mismatching download and returns `HashMismatch`, extracts into
    /// `root/.staging/<pid>-<n>`, writes the receipt there, replaces a receipt-less
    /// package dir, and renames the staging dir into place last.
    ///
    /// `starting` runs under the lock once the package is known to be missing, right
    /// before the download or extraction: the caller announces the install there, so a
    /// package another process has just installed announces nothing.
    pub fn install(
        &self,
        id: &PackageId,
        source: &SourceSpec,
        fetch: &dyn Fetch,
        entry: &ArchiveEntry,
        starting: impl FnOnce(),
    ) -> Result<Receipt> {
        let _lock = self.lock()?;
        if let Some(receipt) = self.installed(id)? {
            return Ok(receipt);
        }
        self.check_placement(id)?;
        starting();
        // Holding the lock, no other install is running: anything staged is left over.
        let staging_root = self.root.join(".staging");
        remove_dir_if_exists(&staging_root)?;
        let url = source.resolve(&entry.url)?;
        let archive = self.cache.archive(id, &url, fetch, entry)?;
        let n = STAGED.fetch_add(1, Ordering::Relaxed);
        let staging = staging_root.join(format!("{}-{n}", std::process::id()));
        fs::create_dir_all(&staging).map_err(io_at(&staging))?;
        let receipt = Receipt {
            id: id.clone(),
            sha256: entry.sha256.clone(),
            source: source.name.clone(),
            url,
        };
        // The receipt goes in before the rename, so the package appears with it at once.
        let staged = TarGz::new(&self.cache.path(entry), &receipt.url)
            .extract_file(archive, &staging)
            .and_then(|()| Self::refuse_own_receipt(&staging, &receipt.url))
            .and_then(|()| receipt.write(&staging));
        if let Err(e) = staged {
            // That error is the one to report; a staging dir that cannot be removed now
            // is removed by the next install.
            let _ = fs::remove_dir_all(&staging);
            return Err(e);
        }
        let dir = self.package_dir(id);
        remove_dir_if_exists(&dir)?;
        if let Some(parent) = dir.parent() {
            fs::create_dir_all(parent).map_err(io_at(parent))?;
        }
        fs::rename(&staging, &dir).map_err(io_at(&dir))?;
        Ok(receipt)
    }

    /// An archive may not bring its own receipt, nor anything under the name the receipt
    /// is written to first: only symdev says a package is installed.
    fn refuse_own_receipt(staging: &Path, url: &str) -> Result<()> {
        for name in [Receipt::FILE, Receipt::PARTIAL] {
            if fs::symlink_metadata(staging.join(name)).is_ok() {
                return Err(SdkError::UnsafeEntry {
                    url: url.to_string(),
                    entry: name.to_string(),
                    reason: "has the name of symdev's package receipt",
                });
            }
        }
        Ok(())
    }

    /// `root/.lock`, held until the returned file is dropped.
    fn lock(&self) -> Result<File> {
        lock(&self.root)
    }
}

/// `dir/.lock` (`dir` created if missing), locked until the returned file is dropped.
fn lock(dir: &Path) -> Result<File> {
    fs::create_dir_all(dir).map_err(io_at(dir))?;
    let path = dir.join(".lock");
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(&path)
        .map_err(io_at(&path))?;
    file.lock().map_err(io_at(&path))?;
    Ok(file)
}

fn remove_dir_if_exists(path: &Path) -> Result<()> {
    match fs::remove_dir_all(path) {
        Err(e) if e.kind() != io::ErrorKind::NotFound => Err(io_at(path)(e)),
        _ => Ok(()),
    }
}

fn remove_file_if_exists(path: &Path) -> Result<()> {
    match fs::remove_file(path) {
        Err(e) if e.kind() != io::ErrorKind::NotFound => Err(io_at(path)(e)),
        _ => Ok(()),
    }
}

fn io_at(path: &Path) -> impl FnOnce(io::Error) -> SdkError + use<> {
    let path = path.display().to_string();
    move |source| SdkError::Io { path, source }
}

mod download_cache;
mod placement;
#[cfg(test)]
mod tests;
