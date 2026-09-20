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

    /// Whether EKA2L1 already holds an executable of this name on drive E. The emulator
    /// refuses to install over one, so `run` uninstalls first when it does. A data
    /// directory we cannot read means "not installed": the worst case is the refusal
    /// that happened before this existed, never a wrong uninstall.
    pub fn installed(exe: Option<&str>) -> bool {
        let (Some(exe), Ok(data)) = (exe, crate::results::EmulatorData::from_env()) else {
            return false;
        };
        data.drive_e().join("sys").join("bin").join(exe).is_file()
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
        let replace = Self::installed(exe_name(sisx).as_deref());
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

/// `<name>.sisx` names `<name>.exe` only when the package name is the binary's name.
/// It is not, whenever an MMP's `TARGET` differs from the manifest's `package.name`, so
/// the real name is read out of the `.pkg` symdev writes beside the SIS.
fn exe_name(sisx: &Path) -> Option<String> {
    let pkg = std::fs::read_to_string(sisx.with_extension("pkg")).ok()?;
    let line = pkg
        .lines()
        .find(|l| l.to_ascii_lowercase().contains("\\sys\\bin\\"))?;
    let (_, after) = line.rsplit_once("\\sys\\bin\\")?;
    Some(after.trim_end_matches(['"', '\r']).to_string())
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
        // Not passed otherwise: `--remove` fails when nothing is installed, and a
        // failing option aborts the invocation before `--install` is reached.
        assert_eq!(
            backend().run_args_replacing(Path::new("/p/build/hello.sisx"), 0xef9f_2cab, false),
            backend().run_args(Path::new("/p/build/hello.sisx"), 0xef9f_2cab)
        );
    }

    #[test]
    fn the_exe_name_comes_from_the_pkg_not_from_the_sisx_name() {
        let dir = tempfile::tempdir().unwrap();
        let sisx = dir.path().join("puzzles.sisx");
        std::fs::write(
            dir.path().join("puzzles.pkg"),
            "&EN\r\n\"Puzzles_0xa000ef77.exe\"\t\t-\"!:\\sys\\bin\\Puzzles_0xa000ef77.exe\"\r\n",
        )
        .unwrap();
        assert_eq!(
            exe_name(&sisx).as_deref(),
            Some("Puzzles_0xa000ef77.exe"),
            "the package name and the binary's name differ whenever an MMP TARGET does"
        );
        assert_eq!(exe_name(&dir.path().join("missing.sisx")), None);
    }
}
