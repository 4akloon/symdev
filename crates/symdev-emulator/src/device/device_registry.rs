//! `DeviceRegistry`: the emulators symdev started, one `<id>.toml` each in
//! `$XDG_RUNTIME_DIR/symdev/devices` (design spec §5). An entry is live while its PID is an
//! EKA2L1 and its socket answers; a dead one is removed, and its process — which may be
//! anything by now — is never signalled.
use std::path::{Path, PathBuf};

use symdev_core::{Error, Result};

use super::{DeviceId, RegistryEntry};

pub struct DeviceRegistry {
    dir: PathBuf,
}

impl DeviceRegistry {
    pub fn at(dir: PathBuf) -> Self {
        Self { dir }
    }

    pub fn from_env() -> Result<Self> {
        match std::env::var_os("XDG_RUNTIME_DIR").filter(|v| !v.is_empty()) {
            Some(run) => Ok(Self::at(PathBuf::from(run).join("symdev/devices"))),
            None => Err(Error::Other(
                "XDG_RUNTIME_DIR is not set: symdev keeps the list of running emulators there \
                 (a login session sets it)"
                    .into(),
            )),
        }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn add(&self, entry: &RegistryEntry) -> Result<()> {
        std::fs::create_dir_all(&self.dir).map_err(|e| file(&self.dir, e))?;
        let path = self.path(&entry.id);
        std::fs::write(&path, entry.to_toml()).map_err(|e| file(&path, e))
    }

    pub fn remove(&self, id: &DeviceId) -> Result<()> {
        let path = self.path(id);
        match std::fs::remove_file(&path) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(file(&path, e)),
            _ => Ok(()),
        }
    }

    /// The lowest `emulator-<n>` no entry has.
    pub fn next_id(&self) -> Result<DeviceId> {
        let taken: Vec<u32> = self.entries()?.iter().map(|e| e.id.number()).collect();
        (1..=u32::MAX)
            .find(|n| !taken.contains(n))
            .and_then(DeviceId::nth)
            .ok_or_else(|| Error::Other("no free emulator id".into()))
    }

    /// The entries whose socket answers, in id order. An entry whose PID is not an EKA2L1
    /// (gone, or reused by another process) is removed; one whose EKA2L1 runs but does not
    /// answer stays registered, so `symdev emulator stop` can still end it. Nothing is
    /// signalled.
    pub fn live(
        &self,
        is_eka2l1: impl Fn(u32) -> bool,
        answers: impl Fn(&RegistryEntry) -> bool,
    ) -> Result<Vec<RegistryEntry>> {
        Ok(self
            .registered(is_eka2l1)?
            .into_iter()
            .filter(|e| answers(e))
            .collect())
    }

    /// The entries whose PID is still an EKA2L1, answering or not, in id order. Every other
    /// entry is removed, and nothing is signalled.
    pub fn registered(&self, is_eka2l1: impl Fn(u32) -> bool) -> Result<Vec<RegistryEntry>> {
        let mut kept = Vec::new();
        for entry in self.entries()? {
            if is_eka2l1(entry.pid) {
                kept.push(entry);
            } else {
                self.remove(&entry.id)?;
            }
        }
        Ok(kept)
    }

    /// Every readable entry, in id order. A file that is not an entry is left alone.
    fn entries(&self) -> Result<Vec<RegistryEntry>> {
        let Ok(dir) = std::fs::read_dir(&self.dir) else {
            return Ok(Vec::new());
        };
        let mut entries = Vec::new();
        for item in dir.flatten() {
            let path = item.path();
            let named = path
                .file_stem()
                .and_then(|s| s.to_str())
                .and_then(DeviceId::parse)
                .is_some();
            if !named || path.extension().is_none_or(|x| x != "toml") {
                continue;
            }
            let text = std::fs::read_to_string(&path).map_err(|e| file(&path, e))?;
            entries.push(RegistryEntry::from_toml(&text, &path)?);
        }
        entries.sort_by_key(|e| e.id);
        Ok(entries)
    }

    fn path(&self, id: &DeviceId) -> PathBuf {
        self.dir.join(format!("{id}.toml"))
    }
}

/// Whether `pid` is an EKA2L1 process now: `/proc/<pid>/comm` names it (`eka2l1_qt`), as
/// symdev has always checked a PID it did not start.
pub fn is_eka2l1(pid: u32) -> bool {
    std::fs::read_to_string(format!("/proc/{pid}/comm"))
        .is_ok_and(|comm| comm.trim().to_ascii_lowercase().contains("eka2l1"))
}

fn file(path: &Path, e: std::io::Error) -> Error {
    Error::Other(format!("{}: {e}", path.display()))
}
