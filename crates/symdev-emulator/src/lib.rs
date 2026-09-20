//! EKA2L1 as a separate process (GPL-3.0: never linked or vendored).

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use symdev_core::{Error, Result};

mod json;
mod results;

pub use results::{EmulatorData, SCHEMA, TestCase, TestReport, await_report};

/// User-installed EKA2L1 (`SYMDEV_EKA2L1`): a binary or wrapper that accepts the
/// observed `--install <sis>` and `--run <uid>` options (experiment 48).
pub struct Eka2l1Backend {
    pub eka2l1: PathBuf,
}

impl Eka2l1Backend {
    pub fn from_env() -> Result<Self> {
        match std::env::var_os("SYMDEV_EKA2L1") {
            Some(v) if !v.is_empty() => Ok(Self {
                eka2l1: PathBuf::from(v),
            }),
            _ => Err(Error::Other(
                "missing emulator: SYMDEV_EKA2L1 (path to eka2l1_qt or a wrapper)".into(),
            )),
        }
    }

    /// One invocation installs to E: and launches by UID3.
    pub fn run_args(&self, sisx: &Path, uid3: u32) -> Vec<String> {
        self.run_args_replacing(sisx, uid3, false)
    }

    /// The same, optionally uninstalling first.
    ///
    /// EKA2L1 will not install over an executable already on drive E: the log says
    /// `Installation done!` and then `Installation of SIS failed`, and what runs is the
    /// **old** binary — a rebuilt application that looks unchanged. Its `--remove <uid>`
    /// undoes the installation, but it fails when nothing is installed, and a failing
    /// option aborts the whole invocation, so it is only passed when the package really
    /// is there ([`Eka2l1Backend::installed`]).
    pub fn run_args_replacing(&self, sisx: &Path, uid3: u32, replace: bool) -> Vec<String> {
        let uid = format!("0x{uid3:08x}");
        let mut args = vec![self.eka2l1.display().to_string()];
        if replace {
            args.extend(["--remove".to_string(), uid.clone()]);
        }
        args.extend([
            "--install".to_string(),
            sisx.display().to_string(),
            "--run".to_string(),
            uid,
        ]);
        args
    }

    /// Whether EKA2L1 has a package with this UID installed.
    ///
    /// Keyed by UID and not by the executable's name, because that is the key
    /// `--remove` takes: an earlier application can leave `sys\\bin\\hello.exe` behind
    /// under a *different* UID, and then removing by name-derived guesswork asks the
    /// emulator to uninstall something that was never installed. `--remove` fails, and a
    /// failing option aborts the whole invocation before `--install` is reached — the
    /// run dies with `Failed to remove package.` and nothing is installed at all.
    ///
    /// The registry is a directory per UID under `sys/install/sisregistry` on the system
    /// drive (observed: 274 entries on `c`, none on `e`, although packages install to E).
    /// Every drive is searched, so a differently configured emulator still works.
    pub fn installed(uid3: u32) -> bool {
        let Ok(data) = crate::results::EmulatorData::from_env() else {
            return false;
        };
        let Ok(drives) = std::fs::read_dir(data.drives()) else {
            return false;
        };
        let uid = format!("{uid3:08x}");
        drives.flatten().any(|drive| {
            drive
                .path()
                .join("sys")
                .join("install")
                .join("sisregistry")
                .join(&uid)
                .is_dir()
        })
    }

    /// PID recorded by an earlier `run` if that process still exists (Linux `/proc`).
    /// EKA2L1 ignores SIGTERM and each `run` starts a new instance, so callers warn.
    pub fn previous(pid_file: &Path) -> Option<u32> {
        let pid: u32 = std::fs::read_to_string(pid_file)
            .ok()?
            .trim()
            .parse()
            .ok()?;
        let comm = std::fs::read_to_string(format!("/proc/{pid}/comm")).ok()?;
        comm.trim()
            .to_ascii_lowercase()
            .contains("eka2l1")
            .then_some(pid)
    }

    /// Start the emulator in the background; its output goes to `log`.
    pub fn run(&self, sisx: &Path, uid3: u32, log: &Path) -> Result<u32> {
        let replace = Self::installed(uid3);
        let args = self.run_args_replacing(sisx, uid3, replace);
        let out =
            std::fs::File::create(log).map_err(|e| Error::Other(format!("create {log:?}: {e}")))?;
        let err = out
            .try_clone()
            .map_err(|e| Error::Other(format!("log {log:?}: {e}")))?;
        let mut cmd = Command::new(&args[0]);
        cmd.args(&args[1..])
            .stdin(Stdio::null())
            .stdout(out)
            .stderr(err);
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            cmd.process_group(0);
        }
        let child = cmd
            .spawn()
            .map_err(|e| Error::Other(format!("start {:?}: {e}", self.eka2l1)))?;
        Ok(child.id())
    }

    /// Ends an instance **this process started** (`kill -9`).
    ///
    /// EKA2L1 ignores `SIGTERM`, so there is no polite signal to try first. Only a pid
    /// that [`Eka2l1Backend::previous`] still recognises as an EKA2L1 is signalled: the
    /// user may have their own emulator open and a stale pid may have been reused by
    /// something else entirely.
    pub fn stop(pid: u32) -> Result<()> {
        let comm = std::fs::read_to_string(format!("/proc/{pid}/comm")).unwrap_or_default();
        if !comm.trim().to_ascii_lowercase().contains("eka2l1") {
            return Err(Error::Other(format!(
                "refusing to kill pid {pid}: it is not an EKA2L1 process any more"
            )));
        }
        let status = Command::new("kill")
            .args(["-9", &pid.to_string()])
            .status()
            .map_err(|e| Error::Other(format!("kill -9 {pid}: {e}")))?;
        if !status.success() {
            return Err(Error::Other(format!("kill -9 {pid} failed: {status}")));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_args_match_experiment_48() {
        let b = Eka2l1Backend {
            eka2l1: PathBuf::from("/home/u/.local/bin/eka2l1"),
        };
        assert_eq!(
            b.run_args(Path::new("/p/build/hello.sisx"), 0xef9f_2cab),
            [
                "/home/u/.local/bin/eka2l1",
                "--install",
                "/p/build/hello.sisx",
                "--run",
                "0xef9f2cab",
            ]
        );
    }

    #[test]
    fn previous_ignores_missing_or_foreign_pid() {
        let dir = std::env::temp_dir().join(format!("symdev-eka2l1-pid-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let pid_file = dir.join("eka2l1.pid");
        assert_eq!(Eka2l1Backend::previous(&pid_file), None);
        // This test process is alive but is not EKA2L1.
        std::fs::write(&pid_file, std::process::id().to_string()).unwrap();
        assert_eq!(Eka2l1Backend::previous(&pid_file), None);
    }

    #[test]
    fn run_reports_missing_binary() {
        let dir = std::env::temp_dir();
        let b = Eka2l1Backend {
            eka2l1: PathBuf::from("/nonexistent/eka2l1"),
        };
        let err = b
            .run(Path::new("x.sisx"), 1, &dir.join("symdev-eka2l1-test.log"))
            .unwrap_err()
            .to_string();
        assert!(err.contains("/nonexistent/eka2l1"), "{err}");
    }
}

#[cfg(test)]
mod reinstall_tests {
    use super::*;

    fn backend() -> Eka2l1Backend {
        Eka2l1Backend {
            eka2l1: PathBuf::from("/usr/bin/eka2l1"),
        }
    }

    #[test]
    fn replacing_uninstalls_before_it_installs() {
        let args =
            backend().run_args_replacing(Path::new("/p/build/hello.sisx"), 0xef9f_2cab, true);
        assert_eq!(
            args,
            [
                "/usr/bin/eka2l1",
                "--remove",
                "0xef9f2cab",
                "--install",
                "/p/build/hello.sisx",
                "--run",
                "0xef9f2cab",
            ]
        );
        // Not passed otherwise: `--remove` fails when the package is not installed, and
        // a failing option aborts the invocation before `--install` is reached.
        assert_eq!(
            backend().run_args_replacing(Path::new("/p/build/hello.sisx"), 0xef9f_2cab, false),
            backend().run_args(Path::new("/p/build/hello.sisx"), 0xef9f_2cab)
        );
    }

    #[test]
    fn installed_is_keyed_by_uid_the_way_remove_is() {
        let dir = tempfile::tempdir().unwrap();
        let reg = dir
            .path()
            .join("data/drives/c/sys/install/sisregistry/e0000685");
        std::fs::create_dir_all(&reg).unwrap();
        // An executable of the same name left by a *different* package is not this
        // package. Keying off the name would ask the emulator to remove a UID it does
        // not have; `--remove` would fail and the run would die before installing.
        let bin = dir.path().join("data/drives/e/sys/bin");
        std::fs::create_dir_all(&bin).unwrap();
        std::fs::write(bin.join("hello.exe"), b"").unwrap();

        // SAFETY: single-threaded test; `EmulatorData::from_env` reads this variable.
        unsafe { std::env::set_var("SYMDEV_EKA2L1_DATA", dir.path()) };
        assert!(Eka2l1Backend::installed(0xe000_0685));
        assert!(!Eka2l1Backend::installed(0xe000_0812));
        unsafe { std::env::remove_var("SYMDEV_EKA2L1_DATA") };
    }
}
