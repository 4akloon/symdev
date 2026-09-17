use crate::{Artifact, Output, Package, PathStyle, Project, RemotePath, Result};

pub trait PlatformBackend {}
pub trait LanguageBackend {}
pub trait SDKBackend {}
pub trait ToolchainBackend {}
pub trait TestBackend {}
pub trait RuntimeBackend {}

#[allow(async_fn_in_trait)]
pub trait ExecutionEnvironment {
    async fn run(&self, cmd: std::process::Command, cwd: &RemotePath) -> Result<Output>;
    async fn push(&self, local: &std::path::Path, remote: &RemotePath) -> Result<()>;
    async fn pull(&self, remote: &RemotePath, local: &std::path::Path) -> Result<()>;
    fn path_style(&self) -> PathStyle;
}

pub trait BuildBackend {
    fn build(&self, project: &Project) -> Result<Vec<Artifact>>;
}

pub trait PackageBackend {
    fn package(&self, artifacts: &[Artifact]) -> Result<Package>;
}

pub trait DeviceTransport {
    fn deliver(&self, package: &Package) -> Result<()>;
}

pub trait EmulatorBackend {
    fn list(&self) -> Result<Vec<String>>;
    fn start(&self, id: &str) -> Result<()>;
    fn stop(&self, id: &str) -> Result<()>;
    fn install(&self, id: &str, package: &Package) -> Result<()>;
    fn launch(&self, id: &str, app: &str) -> Result<()>;
    fn logs(&self, id: &str) -> Result<String>;
}

pub trait DebuggerBackend {
    fn attach(&self, target: &str) -> Result<()>;
    fn breakpoint(&self, spec: &str) -> Result<()>;
    fn r#continue(&self) -> Result<()>;
    fn step(&self) -> Result<()>;
    fn backtrace(&self) -> Result<String>;
}
