use std::fs::{self, File};
use std::io::{self, Seek};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use sha2::{Digest, Sha256};

use super::{io_at, lock, remove_file_if_exists};
use crate::{ArchiveEntry, Fetch, PackageId, Result, SdkError};

/// Makes download names unique within one process.
static DOWNLOADS: AtomicU64 = AtomicU64::new(0);

/// The download cache (`$XDG_CACHE_HOME/symdev/downloads`), which several `SYMDEV_HOME`s
/// may share: verified archives named `<sha256>.tar.gz`. Checking, downloading and
/// renaming happen under `.lock`, so an archive is downloaded once however many installs
/// want it. Nothing ever writes into a cached file: a download goes to a `.part` file of
/// its own and is renamed over the cached name only once verified, so a handle opened on
/// a verified archive keeps reading exactly those bytes.
pub(super) struct DownloadCache {
    dir: PathBuf,
}

impl DownloadCache {
    pub(super) fn new(dir: PathBuf) -> Self {
        DownloadCache { dir }
    }

    /// Where the verified archive of `entry` is kept.
    pub(super) fn path(&self, entry: &ArchiveEntry) -> PathBuf {
        self.dir.join(format!("{}.tar.gz", entry.sha256))
    }

    /// The verified archive of `entry`, open at its start: the cached copy if it has the
    /// right size and hash, else a fresh download of `url`. A mismatching download is
    /// deleted and is `HashMismatch`; a damaged cached copy is replaced, never deleted
    /// under a reader.
    pub(super) fn archive(
        &self,
        id: &PackageId,
        url: &str,
        fetch: &dyn Fetch,
        entry: &ArchiveEntry,
    ) -> Result<File> {
        let hex = |c: char| c.is_ascii_digit() || ('a'..='f').contains(&c);
        if entry.sha256.len() != 64 || !entry.sha256.chars().all(hex) {
            return Err(SdkError::Other(format!(
                "{id}: sha256 `{}` of {url} is not 64 lowercase hex digits; the source's index \
                 is damaged",
                entry.sha256
            )));
        }
        let _lock = lock(&self.dir)?;
        let cached = self.path(entry);
        // A truncated or damaged copy (an interrupted run) is never extracted.
        if let Ok(mut file) = File::open(&cached)
            && file.metadata().is_ok_and(|m| m.len() == entry.size)
            && Self::digest(&mut file, &cached)?.0 == entry.sha256
        {
            file.rewind().map_err(io_at(&cached))?;
            return Ok(file);
        }
        self.remove_stale_parts(entry)?;
        let n = DOWNLOADS.fetch_add(1, Ordering::Relaxed);
        let part = self.dir.join(format!(
            "{}.tar.gz.{}-{n}.part",
            entry.sha256,
            std::process::id()
        ));
        let verified = Self::fetch_verified(id, url, fetch, entry, &part);
        let mut file = match verified {
            Ok(file) => file,
            Err(e) => {
                remove_file_if_exists(&part)?;
                return Err(e);
            }
        };
        fs::rename(&part, &cached).map_err(io_at(&cached))?;
        file.rewind().map_err(io_at(&cached))?;
        Ok(file)
    }

    /// Downloads `url` into `part` and checks it through the handle that is returned.
    fn fetch_verified(
        id: &PackageId,
        url: &str,
        fetch: &dyn Fetch,
        entry: &ArchiveEntry,
        part: &Path,
    ) -> Result<File> {
        fetch.download(url, part)?;
        let mut file = File::open(part).map_err(io_at(part))?;
        let (actual, actual_size) = Self::digest(&mut file, part)?;
        if actual != entry.sha256 || actual_size != entry.size {
            return Err(SdkError::HashMismatch {
                id: id.to_string(),
                url: url.to_string(),
                expected: entry.sha256.clone(),
                actual,
                expected_size: entry.size,
                actual_size,
            });
        }
        Ok(file)
    }

    /// Removes what interrupted downloads of `entry` left (`<sha256>.tar.gz.*.part`): no
    /// download runs without the lock, which the caller holds.
    fn remove_stale_parts(&self, entry: &ArchiveEntry) -> Result<()> {
        let prefix = format!("{}.tar.gz.", entry.sha256);
        for found in fs::read_dir(&self.dir).map_err(io_at(&self.dir))? {
            let name = found.map_err(io_at(&self.dir))?.file_name();
            let name = name.to_string_lossy();
            if name.starts_with(&prefix) && name.ends_with(".part") {
                remove_file_if_exists(&self.dir.join(name.as_ref()))?;
            }
        }
        Ok(())
    }

    /// SHA-256 (hex) and size of what `file` holds from its current position.
    fn digest(file: &mut File, path: &Path) -> Result<(String, u64)> {
        let mut hasher = Sha256::new();
        let size = io::copy(file, &mut hasher).map_err(io_at(path))?;
        Ok((format!("{:x}", hasher.finalize()), size))
    }
}
