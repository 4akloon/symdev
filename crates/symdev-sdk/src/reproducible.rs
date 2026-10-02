use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{self, BufReader};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::PermissionsExt;
use std::path::{Component, Path, PathBuf};

use flate2::{Compression, GzBuilder};
use sha2::{Digest, Sha256};
use tar::{EntryType, Header};

use crate::{Result, SdkError, TarGz};

type Builder = tar::Builder<flate2::write::GzEncoder<File>>;

/// Packs a directory tree so that the same tree always gives the same bytes, and so the
/// same SHA-256: the publisher can then pin an SDK archive's hash in its recipe.
pub struct ReproducibleTarGz;

impl ReproducibleTarGz {
    /// Packs the listed directories of `root` (relative paths, e.g. "epoc32/include"; "."
    /// is all of `root`) into `out`: entries sorted bytewise, mtime 0, uid/gid 0, empty
    /// user/group names, mode 0o755 for dirs and files with any x bit, else 0o644; gzip
    /// header mtime 0, no file name, level 6. The ancestors of a listed directory are
    /// stored as directories. Symlinks are stored as symlinks only if they stay inside;
    /// otherwise an error names them. Returns (sha256 hex, size).
    pub fn pack(root: &Path, include: &[&str], out: &Path) -> Result<(String, u64)> {
        let mut entries = BTreeMap::new();
        for listed in include {
            let rel = Self::relative(listed)?;
            let mut ancestor = PathBuf::new();
            for name in rel.parent().into_iter().flat_map(Path::iter) {
                ancestor.push(name);
                entries.insert(ancestor.as_os_str().as_bytes().to_vec(), ancestor.clone());
            }
            Self::walk(root, &rel, &mut entries)?;
        }
        let file = File::create(out).map_err(io_at(out))?;
        let gz = GzBuilder::new()
            .mtime(0)
            .operating_system(255)
            .write(file, Compression::new(6));
        let mut tar = tar::Builder::new(gz);
        for rel in entries.values() {
            Self::append(&mut tar, root, rel)?;
        }
        tar.into_inner()
            .and_then(|gz| gz.finish())
            .map_err(io_at(out))?;
        let mut hasher = Sha256::new();
        let mut packed = BufReader::new(File::open(out).map_err(io_at(out))?);
        let size = io::copy(&mut packed, &mut hasher).map_err(io_at(out))?;
        Ok((format!("{:x}", hasher.finalize()), size))
    }

    fn relative(listed: &str) -> Result<PathBuf> {
        let mut rel = PathBuf::new();
        for c in Path::new(listed).components() {
            match c {
                Component::Normal(name) => rel.push(name),
                Component::CurDir => {}
                _ => {
                    return Err(SdkError::Other(format!(
                        "cannot pack `{listed}`: list directories relative to the root, \
                         without `..`"
                    )));
                }
            }
        }
        Ok(rel)
    }

    /// Records `rel` and, for a real directory, everything under it; a symlink is a leaf.
    fn walk(root: &Path, rel: &Path, entries: &mut BTreeMap<Vec<u8>, PathBuf>) -> Result<()> {
        let full = root.join(rel);
        let meta = fs::symlink_metadata(&full).map_err(io_at(&full))?;
        if !rel.as_os_str().is_empty() {
            entries.insert(rel.as_os_str().as_bytes().to_vec(), rel.to_path_buf());
        }
        if meta.is_dir() {
            for child in fs::read_dir(&full).map_err(io_at(&full))? {
                let child = child.map_err(io_at(&full))?;
                Self::walk(root, &rel.join(child.file_name()), entries)?;
            }
        }
        Ok(())
    }

    fn append(tar: &mut Builder, root: &Path, rel: &Path) -> Result<()> {
        let full = root.join(rel);
        let meta = fs::symlink_metadata(&full).map_err(io_at(&full))?;
        let mut h = Header::new_gnu();
        h.set_mtime(0);
        h.set_uid(0);
        h.set_gid(0);
        h.set_size(0);
        let kind = meta.file_type();
        let written = if kind.is_dir() {
            h.set_entry_type(EntryType::Directory);
            h.set_mode(0o755);
            tar.append_data(&mut h, rel, io::empty())
        } else if kind.is_symlink() {
            let target = fs::read_link(&full).map_err(io_at(&full))?;
            if let Some(reason) = TarGz::link_problem(root, rel) {
                return Err(SdkError::Other(format!(
                    "cannot pack {}: its symlink to `{}` {reason}; a package may only link \
                     inside itself",
                    rel.display(),
                    target.display()
                )));
            }
            h.set_entry_type(EntryType::Symlink);
            h.set_mode(0o777);
            tar.append_link(&mut h, rel, &target)
        } else if kind.is_file() {
            let x = meta.permissions().mode() & 0o111 != 0;
            h.set_entry_type(EntryType::Regular);
            h.set_mode(if x { 0o755 } else { 0o644 });
            h.set_size(meta.len());
            let data = File::open(&full).map_err(io_at(&full))?;
            tar.append_data(&mut h, rel, BufReader::new(data))
        } else {
            return Err(SdkError::Other(format!(
                "cannot pack {}: not a file, directory or symlink",
                full.display()
            )));
        };
        written.map_err(io_at(&full))
    }
}

fn io_at(path: &Path) -> impl FnOnce(io::Error) -> SdkError + use<> {
    let path = path.display().to_string();
    move |source| SdkError::Io { path, source }
}

#[cfg(test)]
mod tests;
