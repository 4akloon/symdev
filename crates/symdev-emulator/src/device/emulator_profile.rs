//! `EmulatorProfile`: an emulator's own data folder, as Android's AVD (design spec §5;
//! experiment 114 §2). EKA2L1 started with `--data-dir <profile>` keeps its configuration,
//! log, settings and drives there. The ROM and drive Z are a firmware's, the user's or an
//! installed package's, referenced by symbolic links (the emulator reads them and writes
//! neither: experiments 114 §2 and 115 §3); D and E start empty.
use std::path::{Path, PathBuf};

use symdev_core::{Error, Result};

use crate::EmulatorData;
use crate::device::Firmware;

/// What the runner needs from the log: the guest's `RDebug` lines (`Emulated.Stdout`) and
/// the kernel's panic lines (`Kernel`), which the stock filter hides (experiment 114 §2).
const LOG_FILTER: &str = "log-filter: \"*:info Emulated.Stdout:trace Kernel:trace\"";

/// The suffix of a profile's folder while it is being made (`<name>.partial-<pid>`).
const PARTIAL: &str = ".partial-";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmulatorProfile {
    dir: PathBuf,
    name: String,
}

impl EmulatorProfile {
    pub fn at(root: &Path, name: &str) -> Self {
        Self {
            dir: root.join(name),
            name: name.to_string(),
        }
    }

    /// `$XDG_DATA_HOME/symdev/emulators`, else `~/.local/share/symdev/emulators`.
    pub fn root_from_env() -> Result<PathBuf> {
        Self::root_in(
            std::env::var_os("XDG_DATA_HOME")
                .filter(|v| !v.is_empty())
                .map(PathBuf::from),
            std::env::var_os("HOME")
                .filter(|v| !v.is_empty())
                .map(PathBuf::from),
        )
    }

    pub(crate) fn root_in(
        xdg_data_home: Option<PathBuf>,
        home: Option<PathBuf>,
    ) -> Result<PathBuf> {
        match (xdg_data_home, home) {
            (Some(data), _) => Ok(data.join("symdev/emulators")),
            (None, Some(home)) => Ok(home.join(".local/share/symdev/emulators")),
            (None, None) => Err(Error::Other(
                "neither XDG_DATA_HOME nor HOME is set: symdev keeps emulator profiles there"
                    .into(),
            )),
        }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    /// The instance's own log, which it rewrites at start (experiment 114 §2).
    pub fn log_file(&self) -> PathBuf {
        self.dir.join("EKA2L1.log")
    }

    /// The profile as emulator data: drive E (and the test reports on it) is the profile's.
    pub fn data(&self) -> EmulatorData {
        EmulatorData::at(&self.dir)
    }

    /// Makes the profile from `firmware`; an existing profile is never overwritten. From
    /// the user's data: ROM and the whole drive Z linked, `devices.yml`, drive C and
    /// `config.yml` copied (experiment 114 §2). From a package: ROM and drive Z linked
    /// into it, its `device.yml` as `devices.yml`, empty C, D and E, and a `config.yml`
    /// holding only symdev's log filter, every other option at EKA2L1's default
    /// (experiment 115 §3).
    pub fn create(&self, firmware: &Firmware) -> Result<()> {
        if self.dir.symlink_metadata().is_ok() {
            return Err(Error::Other(format!(
                "emulator profile {} already exists at {}",
                self.name,
                self.dir.display()
            )));
        }
        let partial = self.partial();
        // A leftover of this PID's earlier try is ours to clear; another PID's is its own.
        if partial.symlink_metadata().is_ok() {
            std::fs::remove_dir_all(&partial).map_err(|e| file(&partial, e))?;
        }
        let built = match firmware {
            Firmware::UserData { data, name } => self.create_from_user(&partial, data, name),
            Firmware::Package { root, name } => self.create_from_package(&partial, root, name),
        };
        let placed = built
            .and_then(|()| std::fs::rename(&partial, &self.dir).map_err(|e| file(&self.dir, e)));
        match placed {
            Ok(()) => Ok(()),
            // A concurrent first run placed its profile first: the profile exists, which
            // is what this run wanted.
            Err(_) if self.dir.symlink_metadata().is_ok() => {
                let _ = std::fs::remove_dir_all(&partial);
                Ok(())
            }
            Err(e) => {
                let _ = std::fs::remove_dir_all(&partial);
                Err(e)
            }
        }
    }

    /// Where the profile is built before it is renamed into place; `profiles` skips it.
    fn partial(&self) -> PathBuf {
        let mut name = self.dir.as_os_str().to_owned();
        name.push(format!("{PARTIAL}{}", std::process::id()));
        PathBuf::from(name)
    }

    /// Whether a folder of the profiles' root is a half-made profile, not a profile.
    pub fn is_partial(name: &str) -> bool {
        name.contains(PARTIAL)
    }

    /// The profile from the user's EKA2L1 data `from` for `firmware` (a folder of
    /// `data/roms`).
    fn create_from_user(&self, to: &Path, from: &EmulatorData, firmware: &str) -> Result<()> {
        let user = from.root().join("data");
        let rom = user.join("roms").join(firmware);
        if !rom.is_dir() {
            return Err(Error::Other(format!(
                "no firmware {firmware} in {}: install the firmware in EKA2L1 first",
                user.join("roms").display()
            )));
        }
        let data = to.join("data");
        for dir in ["drives/d", "drives/e", "roms"] {
            make_dir(&data.join(dir))?;
        }
        copy_file(&user.join("devices.yml"), &data.join("devices.yml"))?;
        link(&rom, &data.join("roms").join(firmware))?;
        link(&user.join("drives/z"), &data.join("drives/z"))?;
        let c = user.join("drives/c");
        copy_tree(&c, &c, &data.join("drives/c"))?;
        let config = from.root().join("config.yml");
        let text = std::fs::read_to_string(&config).map_err(|e| file(&config, e))?;
        let out = to.join("config.yml");
        std::fs::write(&out, Self::with_log_filter(&text)).map_err(|e| file(&out, e))
    }

    fn create_from_package(&self, to: &Path, root: &Path, name: &str) -> Result<()> {
        let data = to.join("data");
        for dir in ["drives/c", "drives/d", "drives/e", "roms"] {
            make_dir(&data.join(dir))?;
        }
        copy_file(&root.join("device.yml"), &data.join("devices.yml"))?;
        link(&root.join("roms").join(name), &data.join("roms").join(name))?;
        link(&root.join("drives/z"), &data.join("drives/z"))?;
        let out = to.join("config.yml");
        std::fs::write(&out, format!("{LOG_FILTER}\n")).map_err(|e| file(&out, e))
    }

    /// `text` with its `log-filter:` line replaced by symdev's; the user's is kept as a
    /// comment above it. Without one, symdev's is appended.
    fn with_log_filter(text: &str) -> String {
        let mut out = String::new();
        let mut found = false;
        for line in text.lines() {
            if line.starts_with("log-filter:") {
                out.push_str(&format!("# the user's: {line}\n{LOG_FILTER}\n"));
                found = true;
            } else {
                out.push_str(line);
                out.push('\n');
            }
        }
        if !found {
            out.push_str(LOG_FILTER);
            out.push('\n');
        }
        out
    }
}

/// Copies the tree `from` (inside `top`) to `to`. A symbolic link is copied as a link when
/// it points inside `top`; one that leaves it is refused, so the copy never reaches into
/// the rest of the user's files.
fn copy_tree(top: &Path, from: &Path, to: &Path) -> Result<()> {
    make_dir(to)?;
    for entry in std::fs::read_dir(from).map_err(|e| file(from, e))? {
        let entry = entry.map_err(|e| file(from, e))?;
        let (path, target) = (entry.path(), to.join(entry.file_name()));
        let kind = entry.file_type().map_err(|e| file(&path, e))?;
        if kind.is_symlink() {
            let points = std::fs::read_link(&path).map_err(|e| file(&path, e))?;
            let resolved = path.parent().unwrap_or(top).join(&points);
            match resolved.canonicalize() {
                Ok(real) if real.starts_with(top.canonicalize().map_err(|e| file(top, e))?) => {
                    link(&points, &target)?
                }
                _ => {
                    return Err(Error::Other(format!(
                        "{} is a link to {}, outside {}; symdev copies drive C and will not \
                         follow it",
                        path.display(),
                        points.display(),
                        top.display()
                    )));
                }
            }
        } else if kind.is_dir() {
            copy_tree(top, &path, &target)?;
        } else {
            copy_file(&path, &target)?;
        }
    }
    Ok(())
}

fn make_dir(dir: &Path) -> Result<()> {
    std::fs::create_dir_all(dir).map_err(|e| file(dir, e))
}

fn copy_file(from: &Path, to: &Path) -> Result<()> {
    std::fs::copy(from, to)
        .map(|_| ())
        .map_err(|e| Error::Other(format!("copy {} to {}: {e}", from.display(), to.display())))
}

fn link(target: &Path, at: &Path) -> Result<()> {
    std::os::unix::fs::symlink(target, at).map_err(|e| file(at, e))
}

fn file(path: &Path, e: std::io::Error) -> Error {
    Error::Other(format!("{}: {e}", path.display()))
}
