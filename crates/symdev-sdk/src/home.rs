use std::fs::{self, File, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use sha2::{Digest, Sha256};

use crate::{ArchiveEntry, Fetch, PackageId, Receipt, Result, SdkError, SourceSpec, TarGz};

/// How deep `list` looks for receipts: ids of up to this many segments.
const MAX_SEGMENTS: usize = 8;

/// Makes staging names unique within one process.
static STAGED: AtomicU64 = AtomicU64::new(0);

/// The installed packages under `SYMDEV_HOME` (`root`), and the download cache. A package
/// lives at `root/<id path>` and counts as installed only once its receipt is written.
pub struct SdkHome {
    root: PathBuf,
    cache: PathBuf,
}

impl SdkHome {
    pub fn new(root: PathBuf, cache: PathBuf) -> Self {
        SdkHome { root, cache }
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
        let archive = self.download(id, &url, fetch, entry)?;
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
        let staged = TarGz::new(&archive, &receipt.url)
            .extract(&staging)
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

    /// The verified archive in the cache: the cached copy if it has the right size and
    /// hash, else a fresh download (into `.part`, renamed only once verified).
    fn download(
        &self,
        id: &PackageId,
        url: &str,
        fetch: &dyn Fetch,
        entry: &ArchiveEntry,
    ) -> Result<PathBuf> {
        let hex = |c: char| c.is_ascii_digit() || ('a'..='f').contains(&c);
        if entry.sha256.len() != 64 || !entry.sha256.chars().all(hex) {
            return Err(SdkError::Other(format!(
                "{id}: sha256 `{}` of {url} is not 64 lowercase hex digits; the source's index \
                 is damaged",
                entry.sha256
            )));
        }
        fs::create_dir_all(&self.cache).map_err(io_at(&self.cache))?;
        let cached = self.cache.join(format!("{}.tar.gz", entry.sha256));
        // A truncated or damaged copy (an interrupted run) is never extracted.
        if fs::metadata(&cached).is_ok_and(|m| m.len() == entry.size)
            && digest(&cached)?.0 == entry.sha256
        {
            return Ok(cached);
        }
        remove_file_if_exists(&cached)?;
        let part = self.cache.join(format!("{}.tar.gz.part", entry.sha256));
        if let Err(e) = fetch.download(url, &part) {
            remove_file_if_exists(&part)?;
            return Err(e);
        }
        let (actual, actual_size) = digest(&part)?;
        if actual != entry.sha256 || actual_size != entry.size {
            remove_file_if_exists(&part)?;
            return Err(SdkError::HashMismatch {
                id: id.to_string(),
                url: url.to_string(),
                expected: entry.sha256.clone(),
                actual,
                expected_size: entry.size,
                actual_size,
            });
        }
        fs::rename(&part, &cached).map_err(io_at(&cached))?;
        Ok(cached)
    }

    /// `root/.lock`, held until the returned file is dropped.
    fn lock(&self) -> Result<File> {
        fs::create_dir_all(&self.root).map_err(io_at(&self.root))?;
        let path = self.root.join(".lock");
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(&path)
            .map_err(io_at(&path))?;
        file.lock().map_err(io_at(&path))?;
        Ok(file)
    }
}

fn digest(path: &Path) -> Result<(String, u64)> {
    let mut file = File::open(path).map_err(io_at(path))?;
    let mut hasher = Sha256::new();
    let size = io::copy(&mut file, &mut hasher).map_err(io_at(path))?;
    Ok((format!("{:x}", hasher.finalize()), size))
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

mod placement;
#[cfg(test)]
mod tests;
