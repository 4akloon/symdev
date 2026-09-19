//! EKA2L1 as a separate process (GPL-3.0: never linked or vendored).

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use symdev_core::{Error, Result};

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
        vec![
            self.eka2l1.display().to_string(),
            "--install".into(),
            sisx.display().to_string(),
            "--run".into(),
            format!("0x{uid3:08x}"),
        ]
    }

    /// Start the emulator in the background; its output goes to `log`.
    pub fn run(&self, sisx: &Path, uid3: u32, log: &Path) -> Result<u32> {
        let args = self.run_args(sisx, uid3);
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
