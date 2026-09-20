//! `GcceBuild`: running one toolchain command and checking its exit status.
use std::process::Command;

use symdev_core::{Error, RemotePath, Result};

use super::GcceBuild;

impl GcceBuild {
    pub(super) fn run_tool(&self, args: &[String], cwd: &RemotePath) -> Result<()> {
        self.run_tool_env(args, cwd, &[])
    }

    pub(super) fn run_tool_env(
        &self,
        args: &[String],
        cwd: &RemotePath,
        env: &[(&str, String)],
    ) -> Result<()> {
        let mut cmd = Command::new(&args[0]);
        cmd.args(&args[1..]);
        for (k, v) in env {
            cmd.env(k, v);
        }
        let out = self.env.run_blocking(cmd, cwd)?;
        if out.status != 0 {
            return Err(Error::Other(format!(
                "{} failed (status {}): {}",
                args[0],
                out.status,
                String::from_utf8_lossy(&out.stderr)
            )));
        }
        Ok(())
    }
}
