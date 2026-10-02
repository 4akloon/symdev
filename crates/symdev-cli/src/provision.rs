//! Where the toolchain packages live and which ones a command needs (spec §3–§4). The
//! only code that reads `SYMDEV_HOME`, the `XDG_*` directories and the source keys.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::io::Write;
use std::path::PathBuf;

use symdev_build::{Epocroot, Toolchain, ToolchainOverrides};
use symdev_core::Error;
use symdev_manifest::Device;
use symdev_sdk::{Auth, SourceSpec, Sources, builtin_source};
use symdev_sdk::{Gcce, PackageId, Pins, PlatformSdk, S3Keys, SdkHome, SdkManager};

/// The installed packages and the sources, as this process's environment places them;
/// `offline` comes from `--offline`. Each `SYMDEV_*` toolchain variable that is set
/// wins over the packages, field by field.
pub struct Provision {
    offline: bool,
    lookup: Lookup,
}

/// Reads one environment variable (the process's, or a test's map).
type Lookup = Box<dyn Fn(&str) -> Option<OsString>>;

impl Provision {
    pub fn from_env(offline: bool) -> Provision {
        Self::from_lookup(offline, |key| std::env::var_os(key))
    }

    fn from_lookup(
        offline: bool,
        lookup: impl Fn(&str) -> Option<OsString> + 'static,
    ) -> Provision {
        Provision {
            offline,
            lookup: Box::new(lookup),
        }
    }

    pub fn offline(&self) -> bool {
        self.offline
    }

    /// The toolchain for a build: installs what the set variables leave to the packages.
    pub fn toolchain(&self, device: Device) -> Result<Toolchain, Error> {
        let o = self.checked_overrides()?;
        let needed = Self::needed_by(&o, device);
        if needed.is_empty() {
            return Toolchain::resolve(&o, None, None);
        }
        let home = self.install_missing(&needed)?;
        let (gcce_id, sdk_id) = (Pins::gcce(), Pins::platform_sdk(device));
        let gcce = o
            .needs_gcce()
            .then(|| Gcce::at(home.package_dir(&gcce_id), &gcce_id));
        let sdk = o
            .needs_sdk()
            .then(|| PlatformSdk::at(home.package_dir(&sdk_id), &sdk_id));
        Toolchain::resolve(&o, gcce.transpose()?.as_ref(), sdk.transpose()?.as_ref())
    }

    /// The EPOCROOT for reading a `bld.inf`, installing the SDK if it is missing. The
    /// device is asked for only then, so `SYMDEV_EPOCROOT` needs no `symdev.toml`. Only
    /// `SYMDEV_EPOCROOT` is checked: the compiler variables are not this path's business.
    pub fn epocroot(
        &self,
        device: impl FnOnce() -> Result<Device, Error>,
    ) -> Result<Epocroot, Error> {
        let o = self.overrides();
        if !o.needs_sdk() {
            return Epocroot::resolve(&o, None);
        }
        let id = Pins::platform_sdk(device()?);
        let home = self.install_missing(std::slice::from_ref(&id))?;
        let sdk = PlatformSdk::at(home.package_dir(&id), &id)?;
        Epocroot::resolve(&o, Some(&sdk))
    }

    /// The same for `symdev package`, which installs nothing (spec §4): the SDK is
    /// there after `symdev build`, or the error says how to get it.
    pub fn installed_epocroot(&self, device: Device) -> Result<Epocroot, Error> {
        let o = self.overrides();
        if !o.needs_sdk() {
            return Epocroot::resolve(&o, None);
        }
        let id = Pins::platform_sdk(device);
        let home = self.home()?;
        if home.installed(&id)?.is_none() {
            return Err(Error::Other(format!(
                "{id} is not installed, and `symdev package` installs nothing; run `symdev \
                 build` first (it installs what the build needs) or `symdev sdk install {}`, \
                 or set SYMDEV_EPOCROOT to your SDK",
                id.shell_word()
            )));
        }
        let sdk = PlatformSdk::at(home.package_dir(&id), &id)?;
        Epocroot::resolve(&o, Some(&sdk))
    }

    /// The packages, with every id of `ids` installed. The sources, their keys and the
    /// host check matter only to a download, so they are read only for a missing id: a
    /// build whose packages are installed cannot fail on them.
    fn install_missing(&self, ids: &[PackageId]) -> Result<SdkHome, Error> {
        let home = self.home()?;
        let mut missing = Vec::new();
        for id in ids {
            if home.installed(id)?.is_none() {
                missing.push(id.clone());
            }
        }
        if !missing.is_empty() {
            let mut stderr = std::io::stderr();
            self.manager(&mut stderr)?.ensure(&missing)?;
        }
        Ok(home)
    }

    /// The packages a build for `device` needs under the current environment: none for
    /// a part whose every field a `SYMDEV_*` variable sets.
    pub fn needed(&self, device: Device) -> Vec<PackageId> {
        Self::needed_by(&self.overrides(), device)
    }

    fn needed_by(o: &ToolchainOverrides, device: Device) -> Vec<PackageId> {
        let gcce = o.needs_gcce().then(Pins::gcce);
        let sdk = o.needs_sdk().then(|| Pins::platform_sdk(device));
        gcce.into_iter().chain(sdk).collect()
    }

    /// The `SYMDEV_*` toolchain variables that are set.
    fn overrides(&self) -> ToolchainOverrides {
        ToolchainOverrides::from_lookup(|key| (self.lookup)(key))
    }

    /// The same, each set path checked before any download.
    fn checked_overrides(&self) -> Result<ToolchainOverrides, Error> {
        let o = self.overrides();
        o.check()?;
        Ok(o)
    }

    /// The installer: the packages, the sources (built-in first unless `sources.toml`
    /// says `builtin = false`) and the keys of the `s3` ones.
    pub fn manager<'w>(&self, progress: &'w mut dyn Write) -> Result<SdkManager<'w>, Error> {
        let path = self.sources_file()?;
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => Some(text),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => return Err(Error::Other(format!("{}: {e}", path.display()))),
        };
        let path = path.display().to_string();
        let sources = Sources::parse(text.as_deref(), &path, builtin_source().as_ref())?;
        let keys = self.keys(&sources.list)?;
        Ok(SdkManager::new(
            self.home()?,
            sources,
            keys,
            self.offline,
            progress,
        )?)
    }

    /// The installed packages and the download cache.
    pub fn home(&self) -> Result<SdkHome, Error> {
        Ok(SdkHome::new(self.home_dir()?, self.cache_dir()?))
    }

    /// `$SYMDEV_HOME`, else `$XDG_DATA_HOME/symdev`, else `~/.local/share/symdev`.
    fn home_dir(&self) -> Result<PathBuf, Error> {
        match self.var("SYMDEV_HOME") {
            Some(home) if home.is_absolute() => Ok(home),
            Some(home) => Err(Error::Other(format!(
                "SYMDEV_HOME must be an absolute path, not `{}`",
                home.display()
            ))),
            None => self.xdg("XDG_DATA_HOME", ".local/share", "symdev"),
        }
    }

    /// `$XDG_CACHE_HOME/symdev/downloads`, else `~/.cache/symdev/downloads`.
    fn cache_dir(&self) -> Result<PathBuf, Error> {
        self.xdg("XDG_CACHE_HOME", ".cache", "symdev/downloads")
    }

    /// `$XDG_CONFIG_HOME/symdev/sources.toml`, else `~/.config/symdev/sources.toml`.
    fn sources_file(&self) -> Result<PathBuf, Error> {
        self.xdg("XDG_CONFIG_HOME", ".config", "symdev/sources.toml")
    }

    /// `$<variable>/<rest>`, else `$HOME/<fallback>/<rest>`. A relative XDG value is
    /// ignored, as the XDG base directory specification requires.
    fn xdg(&self, variable: &str, fallback: &str, rest: &str) -> Result<PathBuf, Error> {
        if let Some(base) = self.var(variable).filter(|p| p.is_absolute()) {
            return Ok(base.join(rest));
        }
        match self.var("HOME") {
            Some(home) => Ok(home.join(fallback).join(rest)),
            None => Err(Error::Other(format!(
                "cannot tell where symdev keeps its packages: HOME is not set; set HOME, or \
                 SYMDEV_HOME and {variable}"
            ))),
        }
    }

    /// The keys of every `s3` source that has both variables set; one variable without
    /// the other is an error naming the missing one.
    fn keys(&self, sources: &[SourceSpec]) -> Result<BTreeMap<String, S3Keys>, Error> {
        let mut keys = BTreeMap::new();
        for source in sources.iter().filter(|s| s.auth == Auth::S3) {
            let (id_var, secret_var) = S3Keys::variable_names(&source.name);
            let text = |name: &str| {
                (self.lookup)(name)
                    .filter(|v| !v.is_empty())
                    .map(|v| v.to_string_lossy().into_owned())
            };
            let missing = |set: &str, unset: &str| {
                Error::Other(format!(
                    "source `{}`: {set} is set but {unset} is not; set both",
                    source.name
                ))
            };
            match (text(&id_var), text(&secret_var)) {
                (Some(access_key_id), Some(secret_access_key)) => {
                    let pair = S3Keys {
                        access_key_id,
                        secret_access_key,
                    };
                    keys.insert(source.name.clone(), pair);
                }
                (None, None) => {}
                (Some(_), None) => return Err(missing(&id_var, &secret_var)),
                (None, Some(_)) => return Err(missing(&secret_var, &id_var)),
            }
        }
        Ok(keys)
    }

    /// A non-empty variable as a path.
    fn var(&self, name: &str) -> Option<PathBuf> {
        (self.lookup)(name)
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
    }
}

#[cfg(test)]
mod tests;
