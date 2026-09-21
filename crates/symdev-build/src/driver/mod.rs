//! `GcceBuild`: the GCCE build backend (compile, link, elf2e32, resources, icon).
use std::path::{Path, PathBuf};

use symdev_core::{Error, LocalEnv};
use symdev_manifest::IconContainer;

use crate::toolchain::Toolchain;

mod bitmap;
mod build;
mod compile;
mod container;
mod elf2e32_args;
mod gcce_compat;
mod icon;
mod language;
mod libcalls;
mod link;
mod module;
mod resource;
mod rust_build;
mod rust_link;
mod rust_shims;
mod source;
mod tool;
mod ui_build;

pub use compile::{CompileFlags, CompileIncludes};
pub use gcce_compat::GcceCompat;
pub use language::SourceLanguage;
pub use libcalls::LibcallArchive;
pub use module::Module;
pub use rust_build::{APP_VTBL, E32MAIN, RustBuild};

pub struct GcceBuild {
    pub env: LocalEnv,
    pub tools: Toolchain,
    pub uid3: u32,
    pub capabilities: Vec<String>,
    /// `[symbian] icon`, relative to the project root.
    pub icon: Option<PathBuf>,
    /// `[[icons]]`: the containers built before any MMP, since sources include their headers.
    pub icons: Vec<IconContainer>,
    /// `[symbian] secure_id`: overrides an MMP `SECUREID` (experiment 66).
    pub secure_id: Option<u32>,
}

fn arg(path: &Path) -> String {
    path.display().to_string()
}

fn io(err: std::io::Error) -> Error {
    Error::Other(err.to_string())
}

/// An artefact a tool was supposed to produce, or an error naming it and `why` the
/// project may not have got one.
fn produced(path: PathBuf, why: &str) -> symdev_core::Result<PathBuf> {
    if path.is_file() {
        return Ok(path);
    }
    Err(Error::Other(format!(
        "cargo produced no {}: {why}",
        path.display()
    )))
}

impl GcceBuild {
    pub(super) fn linkas_for(module: &Module, name: &str) -> String {
        format!("{name}{{000a0000}}[{:08x}].{}", module.uid3, module.ext())
    }

    pub(super) fn exe_module(&self) -> Module {
        Module {
            dll: false,
            uid2: 0,
            uid3: self.uid3,
            allow_data: false,
            secureid: None,
        }
    }
}

#[cfg(test)]
mod tests;
