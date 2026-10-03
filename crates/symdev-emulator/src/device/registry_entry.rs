//! `RegistryEntry`: one running emulator symdev started, as `<id>.toml` in the registry.
use std::path::{Path, PathBuf};

use symdev_core::{Error, Result};

use super::DeviceId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistryEntry {
    pub id: DeviceId,
    pub pid: u32,
    /// The profile it runs on (`EmulatorProfile::name`).
    pub profile: String,
    /// What the emulator reports, `"<manufacturer> <model> (<firmware>)"`.
    pub name: String,
    pub socket: PathBuf,
    pub log: PathBuf,
}

impl RegistryEntry {
    pub fn to_toml(&self) -> String {
        let mut t = toml::Table::new();
        t.insert("id".into(), self.id.to_string().into());
        t.insert("pid".into(), i64::from(self.pid).into());
        t.insert("profile".into(), self.profile.clone().into());
        t.insert("name".into(), self.name.clone().into());
        t.insert("socket".into(), self.socket.display().to_string().into());
        t.insert("log".into(), self.log.display().to_string().into());
        t.to_string()
    }

    /// The entry in `text`; `path` only names it in the error.
    pub fn from_toml(text: &str, path: &Path) -> Result<Self> {
        let bad = |why: &str| Error::Other(format!("{}: {why}", path.display()));
        let t: toml::Table = text
            .parse()
            .map_err(|e: toml::de::Error| bad(&e.to_string()))?;
        let text_of = |key: &str| {
            t.get(key)
                .and_then(|v| v.as_str())
                .map(String::from)
                .ok_or_else(|| bad(&format!("no `{key}`")))
        };
        let id = DeviceId::parse(&text_of("id")?).ok_or_else(|| bad("`id` is not emulator-<n>"))?;
        let pid = t
            .get("pid")
            .and_then(|v| v.as_integer())
            .and_then(|p| u32::try_from(p).ok())
            .ok_or_else(|| bad("no `pid`"))?;
        Ok(Self {
            id,
            pid,
            profile: text_of("profile")?,
            name: text_of("name")?,
            socket: PathBuf::from(text_of("socket")?),
            log: PathBuf::from(text_of("log")?),
        })
    }
}
