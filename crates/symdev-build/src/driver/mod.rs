//! `GcceBuild`: the GCCE build backend (compile, link, elf2e32, resources, icon).
use std::path::{Path, PathBuf};

use symdev_core::{Error, LocalEnv};

use crate::toolchain::Toolchain;

mod build;
mod compile;
mod elf2e32_args;
mod gcce_compat;
mod icon;
mod language;
mod link;
mod module;
mod resource;
mod source;
mod tool;

pub use compile::CompileIncludes;
pub use gcce_compat::GcceCompat;
pub use language::SourceLanguage;
pub use module::Module;

pub struct GcceBuild {
    pub env: LocalEnv,
    pub tools: Toolchain,
    pub uid3: u32,
    pub capabilities: Vec<String>,
    /// `[symbian] icon`, relative to the project root.
    pub icon: Option<PathBuf>,
}

fn arg(path: &Path) -> String {
    path.display().to_string()
}

fn io(err: std::io::Error) -> Error {
    Error::Other(err.to_string())
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
        }
    }
}

#[cfg(test)]
mod tests;
