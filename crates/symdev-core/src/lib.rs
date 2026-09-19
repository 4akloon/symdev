mod capability;
mod error;
mod local_env;
mod traits;
mod types;

pub use capability::Capabilities;
pub use error::*;
pub use local_env::*;
pub use traits::*;
pub use types::*;

#[cfg(test)]
extern crate self as symdev_core;

#[test]
fn not_implemented_error_displays_feature_and_milestone() {
    let err = symdev_core::Error::NotImplemented {
        feature: "symdev build",
        milestone: "M1",
    };
    assert_eq!(
        err.to_string(),
        "not implemented: 'symdev build' (unlocks at M1)"
    );
}

#[test]
fn remote_path_debug_and_display() {
    let p = symdev_core::RemotePath::new("/tmp/hello");
    assert!(format!("{p}").contains("/tmp/hello"));
    assert!(format!("{p:?}").contains("/tmp/hello"));
}

#[test]
fn marker_and_signed_traits_exist() {
    fn assert_marker<T: ?Sized>() {}
    assert_marker::<dyn symdev_core::PlatformBackend>();
    assert_marker::<dyn symdev_core::LanguageBackend>();
    assert_marker::<dyn symdev_core::SDKBackend>();
    assert_marker::<dyn symdev_core::ToolchainBackend>();
    assert_marker::<dyn symdev_core::TestBackend>();
    assert_marker::<dyn symdev_core::RuntimeBackend>();
    let _ = (
        NoopEnv,
        NoopBuild,
        NoopPackage,
        NoopDevice,
        NoopEmulator,
        NoopDebugger,
    );
}

#[cfg(test)]
struct NoopEnv;
#[cfg(test)]
impl symdev_core::ExecutionEnvironment for NoopEnv {
    async fn run(
        &self,
        _cmd: std::process::Command,
        _cwd: &symdev_core::RemotePath,
    ) -> symdev_core::Result<symdev_core::Output> {
        Err(symdev_core::Error::NotImplemented {
            feature: "ExecutionEnvironment::run",
            milestone: "M1",
        })
    }
    async fn push(
        &self,
        _local: &std::path::Path,
        _remote: &symdev_core::RemotePath,
    ) -> symdev_core::Result<()> {
        Ok(())
    }
    async fn pull(
        &self,
        _remote: &symdev_core::RemotePath,
        _local: &std::path::Path,
    ) -> symdev_core::Result<()> {
        Ok(())
    }
    fn path_style(&self) -> symdev_core::PathStyle {
        symdev_core::PathStyle::Posix
    }
}

#[cfg(test)]
struct NoopBuild;
#[cfg(test)]
impl symdev_core::BuildBackend for NoopBuild {
    fn build(
        &self,
        _project: &symdev_core::Project,
    ) -> symdev_core::Result<Vec<symdev_core::Artifact>> {
        Ok(Vec::new())
    }
}

#[cfg(test)]
struct NoopPackage;
#[cfg(test)]
impl symdev_core::PackageBackend for NoopPackage {
    fn package(
        &self,
        _artifacts: &[symdev_core::Artifact],
    ) -> symdev_core::Result<symdev_core::Package> {
        Ok(symdev_core::Package {
            primary: std::path::PathBuf::new(),
            companions: Vec::new(),
        })
    }
}

#[cfg(test)]
struct NoopDevice;
#[cfg(test)]
impl symdev_core::DeviceTransport for NoopDevice {
    fn deliver(&self, _package: &symdev_core::Package) -> symdev_core::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
struct NoopEmulator;
#[cfg(test)]
impl symdev_core::EmulatorBackend for NoopEmulator {
    fn list(&self) -> symdev_core::Result<Vec<String>> {
        Ok(Vec::new())
    }
    fn start(&self, _id: &str) -> symdev_core::Result<()> {
        Ok(())
    }
    fn stop(&self, _id: &str) -> symdev_core::Result<()> {
        Ok(())
    }
    fn install(&self, _id: &str, _package: &symdev_core::Package) -> symdev_core::Result<()> {
        Ok(())
    }
    fn launch(&self, _id: &str, _app: &str) -> symdev_core::Result<()> {
        Ok(())
    }
    fn logs(&self, _id: &str) -> symdev_core::Result<String> {
        Ok(String::new())
    }
}

#[cfg(test)]
struct NoopDebugger;
#[cfg(test)]
impl symdev_core::DebuggerBackend for NoopDebugger {
    fn attach(&self, _target: &str) -> symdev_core::Result<()> {
        Ok(())
    }
    fn breakpoint(&self, _spec: &str) -> symdev_core::Result<()> {
        Ok(())
    }
    fn r#continue(&self) -> symdev_core::Result<()> {
        Ok(())
    }
    fn step(&self) -> symdev_core::Result<()> {
        Ok(())
    }
    fn backtrace(&self) -> symdev_core::Result<String> {
        Ok(String::new())
    }
}

#[test]
fn local_env_run_true_status_zero() {
    let out = LocalEnv
        .run_blocking(std::process::Command::new("true"), &RemotePath::new("/"))
        .unwrap();
    assert_eq!(out.status, 0);
}

#[test]
fn local_env_push_copies_file() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("a.txt");
    let dst = dir.path().join("b.txt");
    std::fs::write(&src, b"hi").unwrap();
    LocalEnv
        .push_blocking(&src, &RemotePath::new(dst.to_str().unwrap()))
        .unwrap();
    assert_eq!(std::fs::read(dst).unwrap(), b"hi");
}
