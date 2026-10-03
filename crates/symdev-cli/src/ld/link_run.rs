//! `LinkRun`: one `symdev-ld` call — link, package, and leave the files cargo and the
//! runner look for (design spec §4; experiment 114 §1).
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use symdev_build::RustcLink;
use symdev_core::{Error, Project, Result};

use super::{CargoLinkEnv, CargoOutput, LinkKind, LinkRecord, LinkerArgs};
use crate::provision::Provision;
use crate::rust_project::RustProject;
use crate::sisx::ProjectPackage;

pub(crate) struct LinkRun {
    args: LinkerArgs,
    env: CargoLinkEnv,
}

impl LinkRun {
    pub fn from_env(args: impl Iterator<Item = OsString>) -> Result<Self> {
        let args = LinkerArgs::parse(args)?;
        Ok(Self {
            args,
            env: CargoLinkEnv::from_pairs(std::env::vars()),
        })
    }

    /// This link's own directory, beside rustc's output.
    pub fn work_for(args: &LinkerArgs) -> Result<PathBuf> {
        Ok(CargoOutput::of(&args.output)?.work_dir())
    }

    pub fn run(&self) -> Result<()> {
        let root =
            self.env.manifest_dir.clone().ok_or_else(|| {
                Error::Other(
            "symdev-ld: CARGO_MANIFEST_DIR is not set: symdev-ld is cargo's linker, named in \
             .cargo/config.toml; run `cargo build`".into())
            })?;
        let manifest = symdev_manifest::load(root.join("symdev.toml")).map_err(|e| {
            Error::Other(format!(
                "symdev-ld: no usable symdev.toml in {} ({e}); `symdev new --lang rust` makes one",
                root.display()
            ))
        })?;
        if !manifest.language.is_rust() {
            return Err(Error::Other(format!(
                "symdev-ld: {} is not a Rust project",
                root.display()
            )));
        }
        let kind = LinkKind::of(&self.env, &manifest.package.name)?;
        let out = CargoOutput::of(&self.args.output)?;
        for dir in &self.args.raw_dylibs {
            if std::fs::read_dir(dir).is_ok_and(|mut d| d.next().is_some()) {
                return Err(Error::Other(format!(
                    "symdev-ld: rustc's {} is not empty: TODO: \
                    raw-dylib imports (not observed)",
                    dir.display()
                )));
            }
        }
        let provision = Provision::from_env(false);
        let rust = RustProject::resolve(&manifest, &root, &provision, kind == LinkKind::Main)?;
        let link = RustcLink {
            inputs: self.args.inputs.clone(),
            work: Self::work_for(&self.args)?,
        };
        let artifacts = rust
            .build
            .link_rustc_output(&Project { root: root.clone() }, &link)?;
        let exe = &artifacts
            .first()
            .ok_or_else(|| Error::Other("symdev-ld: no image".into()))?
            .path;
        let package = ProjectPackage::new(manifest.clone(), root.clone(), rust.epocroot.clone())?;
        let password = std::env::var("SYMDEV_SIGN_PASSWORD").unwrap_or_default(); // per D1
        let sisx = package.package(exe, &password)?;
        copy(exe, out.path())?;
        copy(&sisx, &out.sisx())?;
        LinkRecord { kind: kind.clone() }.write(&out.record())?;
        if kind == LinkKind::Main {
            let bin = out
                .path()
                .file_name()
                .map(PathBuf::from)
                .unwrap_or_default();
            let beside = out.profile_dir().join(&bin);
            copy(&sisx, &beside.with_extension("sisx"))?;
            LinkRecord { kind }
                .write(&PathBuf::from(format!("{}.symdev.toml", beside.display())))?;
            let build = root.join("build");
            copy(exe, &build.join(format!("{}.exe", package.app())))?;
            copy(
                &sisx,
                &build.join(format!("{}.sisx", manifest.package.name)),
            )?;
        }
        Ok(())
    }
}

fn copy(from: &Path, to: &Path) -> Result<()> {
    std::fs::copy(from, to).map(|_| ()).map_err(|e| {
        Error::Other(format!(
            "symdev-ld: copy {} to {}: {e}",
            from.display(),
            to.display()
        ))
    })
}
