use std::collections::VecDeque;
use std::ffi::OsString;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufReader};
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Component, Path, PathBuf};

use flate2::read::GzDecoder;
use tar::EntryType;

use crate::{Result, SdkError};

/// How many symlinks one lookup may follow, as Linux allows (`MAXSYMLINKS`).
const MAX_HOPS: u32 = 40;

/// A downloaded `.tar.gz` package archive; `url` is where it came from, for messages.
pub struct TarGz<'a> {
    path: &'a Path,
    url: &'a str,
}

impl<'a> TarGz<'a> {
    pub fn new(path: &'a Path, url: &'a str) -> Self {
        TarGz { path, url }
    }

    /// Extracts into `into` (must exist, empty). Refuses absolute paths, `..`, device and
    /// fifo entries, symlinks and hard links whose target resolves outside `into`. Keeps
    /// the executable bit. Files and directories are written first; links are created
    /// after them and never written through, and every symlink is then followed the way
    /// the kernel would, so a chain of links cannot leave `into` either. On error `into`
    /// may hold part of the package; the caller removes it.
    pub fn extract(&self, into: &Path) -> Result<()> {
        let io_at = |path: &Path| {
            let path = path.display().to_string();
            move |source| SdkError::Io { path, source }
        };
        let mut dir = fs::read_dir(into).map_err(io_at(into))?;
        if dir.next().is_some() {
            return Err(SdkError::Other(format!(
                "{} must be an empty directory to extract {} into",
                into.display(),
                self.url
            )));
        }
        let file = File::open(self.path).map_err(io_at(self.path))?;
        let mut archive = tar::Archive::new(GzDecoder::new(BufReader::new(file)));
        let mut links = Vec::new();
        for entry in archive.entries().map_err(io_at(self.path))? {
            let mut entry = entry.map_err(io_at(self.path))?;
            let raw = entry.path().map_err(io_at(self.path))?.into_owned();
            let rel = self.relative(&raw, &raw)?;
            match entry.header().entry_type() {
                EntryType::Directory => {
                    self.real_dir(into, &rel, &raw)?;
                }
                EntryType::Regular | EntryType::Continuous => {
                    let dest = self.slot(into, &rel, &raw)?;
                    let x = entry.header().mode().map_err(io_at(self.path))? & 0o111 != 0;
                    let mut out = OpenOptions::new()
                        .write(true)
                        .create_new(true)
                        .mode(if x { 0o755 } else { 0o644 })
                        .open(&dest)
                        .map_err(io_at(&dest))?;
                    io::copy(&mut entry, &mut out).map_err(io_at(&dest))?;
                }
                kind @ (EntryType::Symlink | EntryType::Link) => {
                    let target = entry.link_name().map_err(io_at(self.path))?;
                    let target =
                        target.ok_or_else(|| self.unsafe_entry(&raw, "has no link target"))?;
                    links.push((raw.clone(), rel, target.into_owned(), kind));
                }
                EntryType::Char | EntryType::Block | EntryType::Fifo => {
                    return Err(self.unsafe_entry(&raw, "is a device or a fifo"));
                }
                _ => return Err(self.unsafe_entry(&raw, "has an unsupported type")),
            }
        }
        for (raw, rel, target, kind) in &links {
            let dest = self.slot(into, rel, raw)?;
            if *kind == EntryType::Symlink {
                if target.has_root() {
                    return Err(self.unsafe_entry(raw, "links outside the package"));
                }
                std::os::unix::fs::symlink(target, &dest).map_err(io_at(&dest))?;
            } else {
                let source = self.real_file(into, &self.relative(target, raw)?, raw)?;
                fs::hard_link(&source, &dest).map_err(io_at(&dest))?;
            }
        }
        for (raw, rel, _, kind) in &links {
            let problem = (*kind == EntryType::Symlink).then(|| Self::link_problem(into, rel));
            if let Some(reason) = problem.flatten() {
                return Err(self.unsafe_entry(raw, reason));
            }
        }
        Ok(())
    }

    /// `path` without `.` components; refuses absolute paths and `..`.
    fn relative(&self, path: &Path, raw: &Path) -> Result<PathBuf> {
        let mut rel = PathBuf::new();
        for c in path.components() {
            match c {
                Component::Normal(name) => rel.push(name),
                Component::CurDir => {}
                Component::ParentDir => return Err(self.unsafe_entry(raw, "has a `..` component")),
                Component::RootDir | Component::Prefix(_) => {
                    return Err(self.unsafe_entry(raw, "is an absolute path"));
                }
            }
        }
        Ok(rel)
    }

    /// Where a file or link entry goes: `into/rel`, its parent made a real directory.
    fn slot(&self, into: &Path, rel: &Path, raw: &Path) -> Result<PathBuf> {
        let name = rel
            .file_name()
            .ok_or_else(|| self.unsafe_entry(raw, "has an empty path"))?;
        let parent = rel.parent().unwrap_or(Path::new(""));
        Ok(self.real_dir(into, parent, raw)?.join(name))
    }

    /// Creates `into/rel` one directory at a time, refusing to pass through anything
    /// that is not a real directory (a symlink created by an earlier entry, or a file).
    fn real_dir(&self, into: &Path, rel: &Path, raw: &Path) -> Result<PathBuf> {
        let mut here = into.to_path_buf();
        for name in rel.iter() {
            here.push(name);
            match fs::symlink_metadata(&here) {
                Ok(m) if m.is_dir() => {}
                Ok(_) => return Err(self.unsafe_entry(raw, "is inside a symlink or a file")),
                Err(e) if e.kind() == io::ErrorKind::NotFound => {
                    fs::create_dir(&here).map_err(|source| SdkError::Io {
                        path: here.display().to_string(),
                        source,
                    })?;
                }
                Err(source) => {
                    let path = here.display().to_string();
                    return Err(SdkError::Io { path, source });
                }
            }
        }
        Ok(here)
    }

    /// `into/rel` if it is a regular file reached only through real directories.
    fn real_file(&self, into: &Path, rel: &Path, raw: &Path) -> Result<PathBuf> {
        let not_a_file =
            || self.unsafe_entry(raw, "is a hard link to something that is not a file");
        let mut here = into.to_path_buf();
        let names: Vec<_> = rel.iter().collect();
        for (n, name) in names.iter().enumerate() {
            here.push(name);
            let m = fs::symlink_metadata(&here).map_err(|_| not_a_file())?;
            let last = n + 1 == names.len();
            if (last && !m.is_file()) || (!last && !m.is_dir()) {
                return Err(not_a_file());
            }
        }
        if names.is_empty() {
            Err(not_a_file())
        } else {
            Ok(here)
        }
    }

    /// Why the symlink `root/rel` is unsafe, if it is: followed component by component the
    /// way the kernel does, substituting every symlink met on the way, the walk must never
    /// climb above `root` nor meet an absolute target. Also used by the packer.
    pub(crate) fn link_problem(root: &Path, rel: &Path) -> Option<&'static str> {
        let mut at: Vec<OsString> = rel.iter().map(OsString::from).collect();
        let mut todo: VecDeque<OsString> = at.pop().into_iter().collect();
        let mut hops = 0;
        while let Some(name) = todo.pop_front() {
            if name == ".." {
                if at.pop().is_none() {
                    return Some("links outside the package");
                }
                continue;
            }
            if name == "." || name.is_empty() {
                continue;
            }
            at.push(name);
            let Ok(target) = fs::read_link(root.join(at.iter().collect::<PathBuf>())) else {
                continue;
            };
            hops += 1;
            if hops > MAX_HOPS {
                return Some("has a symlink loop");
            }
            if target.has_root() {
                return Some("links outside the package");
            }
            at.pop();
            for c in target.iter().rev() {
                todo.push_front(c.to_os_string());
            }
        }
        None
    }

    fn unsafe_entry(&self, raw: &Path, reason: &'static str) -> SdkError {
        SdkError::UnsafeEntry {
            url: self.url.to_string(),
            entry: raw.display().to_string(),
            reason,
        }
    }
}

#[cfg(test)]
mod tests;
