//! `RustcLink`: a link of what rustc compiled, run by `symdev-ld` (design spec §4).
use std::path::PathBuf;

use symdev_core::{Artifact, Project, RemotePath, Result};

use super::{LinkInputs, RustBuild, arg, produced};
use crate::RustLinker;
use crate::file_error;
use crate::required_capability::RequiredCapability;

/// One link's inputs and its own directory. `cargo test` links the binary and every test
/// at once, so nothing a link writes may be shared with another (experiment 114 §1.1).
pub struct RustcLink {
    /// rustc's objects and rlibs, in rustc's order (experiment 114 §1.2).
    pub inputs: Vec<PathBuf>,
    /// Shims, both ELFs, the import stubs, the map, the image and its resources.
    pub work: PathBuf,
}

impl RustBuild {
    /// 0.3.0's link after cargo, on rustc's inputs: the shims, the libcall archive (built
    /// by the same `cargo rustc` as 0.3.0, now nested in cargo's own build: experiment
    /// 114 §1.3 saw no lock wait), rust-lld's two links or GNU ld, the capability check,
    /// elf2e32, and the resources. The image is `work/<name>.exe`.
    pub fn link_rustc_output(&self, project: &Project, link: &RustcLink) -> Result<Vec<Artifact>> {
        std::fs::create_dir_all(&link.work).map_err(|e| file_error(&link.work, e))?;
        let cwd = RemotePath::new(arg(&project.root));
        let lld = match &self.linker {
            RustLinker::Lld { rust_lld, cache } => {
                Some((self.rust_lld_ready(rust_lld.as_deref(), &cwd)?, cache))
            }
            RustLinker::Gnu => None,
        };
        let prebuilt = self.linker.prebuilt(&self.sdk)?;
        let shims = self.shim_archives(&cwd, prebuilt.as_ref(), &link.work)?;
        self.run_cargo_args(&self.libcalls().cargo_args(), &cwd)?;
        let libcalls = produced(
            self.libcalls().path(project),
            "the Rust SDK's symbian-libcalls crate defines the __atomic_* family and memcmp",
        )?;
        let elf = link.work.join(format!("{}.elf", self.name));
        let map = link.work.join(format!("{}.exe.map", self.name));
        match lld {
            None => self.gcce.run_tool(
                &self.link_args(
                    &link.inputs,
                    shims.first().map(PathBuf::as_path),
                    Some(&libcalls),
                    &elf,
                    &map,
                )?,
                &cwd,
            )?,
            Some((rust_lld, cache)) => self.link_lld(
                &rust_lld,
                cache,
                prebuilt.as_ref(),
                &LinkInputs {
                    rust: &link.inputs,
                    shims: &shims,
                    libcalls: &libcalls,
                },
                &elf,
                &map,
                &cwd,
            )?,
        }
        RequiredCapability::check(&elf, &self.gcce.capabilities, &format!("{}.exe", self.name))?;
        let out = link.work.join(format!("{}.exe", self.name));
        self.gcce
            .run_elf2e32(&self.gcce.elf2e32_args(&self.name, &elf, &out), &cwd)?;
        let mut artifacts = vec![Artifact::exe(out)];
        artifacts.extend(self.build_ui(&link.work)?);
        artifacts.extend(self.build_strings(&project.root, &link.work)?);
        Ok(artifacts)
    }
}
