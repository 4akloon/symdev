use crate::{Error, ExecutionEnvironment, Output, PathStyle, RemotePath, Result};

pub struct LocalEnv;

impl LocalEnv {
    pub fn run_blocking(&self, mut cmd: std::process::Command, cwd: &RemotePath) -> Result<Output> {
        let out = cmd
            .current_dir(cwd.to_string())
            .output()
            .map_err(|e| Error::Other(e.to_string()))?;
        Ok(Output {
            status: out.status.code().unwrap_or(1),
            stdout: out.stdout,
            stderr: out.stderr,
        })
    }

    pub fn push_blocking(&self, local: &std::path::Path, remote: &RemotePath) -> Result<()> {
        std::fs::copy(local, remote.to_string())
            .map(|_| ())
            .map_err(|e| Error::Other(e.to_string()))
    }

    pub fn pull_blocking(&self, remote: &RemotePath, local: &std::path::Path) -> Result<()> {
        std::fs::copy(remote.to_string(), local)
            .map(|_| ())
            .map_err(|e| Error::Other(e.to_string()))
    }
}

impl ExecutionEnvironment for LocalEnv {
    async fn run(&self, cmd: std::process::Command, cwd: &RemotePath) -> Result<Output> {
        self.run_blocking(cmd, cwd)
    }
    async fn push(&self, local: &std::path::Path, remote: &RemotePath) -> Result<()> {
        self.push_blocking(local, remote)
    }
    async fn pull(&self, remote: &RemotePath, local: &std::path::Path) -> Result<()> {
        self.pull_blocking(remote, local)
    }
    fn path_style(&self) -> PathStyle {
        PathStyle::Posix
    }
}
