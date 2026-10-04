//! `StdSysroot`: the sysroot `symdev-rustc` points rustc at (experiment 114 §1.4, H4).
//!
//! cargo resolves `-Zbuild-std`'s source from rustc's sysroot before any build script
//! runs, and a variable in `.cargo/config.toml`'s `[env]` does not reach cargo itself
//! (H1). So a `language = "rust-std"` project names `build/symdev-rustc` as `[build]
//! rustc`, and that wrapper runs rustc with `--sysroot build/sysroot`:
//!
//! * `lib/rustlib/src/rust/library`: the patched copy [`StdSrc`] makes;
//! * `lib/rustlib/<host>`: a link to the pinned toolchain's, for the host's own crates
//!   (proc macros, build scripts).
//!
//! It needs neither rustup state nor an absolute path in the project.
use std::path::{Path, PathBuf};
use std::process::Command;

use symdev_core::{Error, Result};

use crate::rust_lld::RustLld;
use crate::rust_sdk::RustSdk;
use crate::std_src::StdSrc;

pub struct StdSysroot {
    dir: PathBuf,
}

impl StdSysroot {
    /// The link `[build] rustc` names, beside the sysroot.
    pub const WRAPPER: &'static str = "symdev-rustc";

    /// Makes `<project>/build/sysroot` afresh and the link `<project>/build/symdev-rustc`
    /// to the running `symdev`. `rustc` is the pinned nightly's, whose `rust-src` is
    /// copied and whose host libraries are linked.
    pub fn materialise(sdk: &RustSdk, rustc: &Path, project_root: &Path) -> Result<Self> {
        let build = project_root.join("build");
        let dir = build.join("sysroot");
        if dir.exists() {
            std::fs::remove_dir_all(&dir).map_err(|e| file(&dir, e))?;
        }
        StdSrc::materialise_into(sdk, rustc, project_root, &dir.join("lib/rustlib/src/rust"))?;
        let sysroot = Self::rustc_says(rustc, project_root, &["--print", "sysroot"])?;
        let version = Self::rustc_says(rustc, project_root, &["-vV"])?;
        let lld = RustLld::in_sysroot(&sysroot, &version)?;
        // `<sysroot>/lib/rustlib/<host>/bin/rust-lld` → `<sysroot>/lib/rustlib/<host>`.
        let host = lld
            .path()
            .parent()
            .and_then(Path::parent)
            .ok_or_else(|| Error::Other("`rustc -vV` named no host directory".into()))?;
        let name = host
            .file_name()
            .ok_or_else(|| Error::Other("`rustc -vV` named no host directory".into()))?;
        symlink(host, &dir.join("lib/rustlib").join(name))?;
        let exe = std::env::current_exe()
            .map_err(|e| Error::Other(format!("where is this symdev? {e}")))?;
        symlink(&exe, &build.join(Self::WRAPPER))?;
        Ok(Self { dir })
    }

    /// `<project>/build/sysroot`.
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    fn rustc_says(rustc: &Path, project_root: &Path, args: &[&str]) -> Result<String> {
        let mut cmd = Command::new(rustc);
        cmd.args(args).current_dir(project_root);
        // As `StdSrc::toolchain_library`: the project's rust-toolchain.toml picks the
        // nightly, not the toolchain symdev itself was started through.
        cmd.env_remove("RUSTUP_TOOLCHAIN");
        let out = cmd.output().map_err(|e| {
            Error::Other(format!(
                "could not run {} ({e}); set SYMDEV_RUSTC",
                rustc.display()
            ))
        })?;
        if !out.status.success() {
            return Err(Error::Other(format!(
                "{} {} failed: {}",
                rustc.display(),
                args.join(" "),
                String::from_utf8_lossy(&out.stderr)
            )));
        }
        Ok(String::from_utf8_lossy(&out.stdout).into_owned())
    }
}

/// A symbolic link `link` → `target`, replacing whatever `link` was.
fn symlink(target: &Path, link: &Path) -> Result<()> {
    if link.symlink_metadata().is_ok() {
        std::fs::remove_file(link).map_err(|e| file(link, e))?;
    }
    std::os::unix::fs::symlink(target, link).map_err(|e| file(link, e))
}

fn file(path: &Path, e: std::io::Error) -> Error {
    Error::Other(format!("{}: {e}", path.display()))
}

#[cfg(test)]
mod tests;
