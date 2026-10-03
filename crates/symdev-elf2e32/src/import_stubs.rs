//! `ImportStubs`: 8-byte import stubs for an lld link (experiment 112).
use crate::{ElfImage, ElfLinker};
use symdev_core::{Error, Result};

mod object;

/// The import stubs GNU ld's symbianelf PLT holds, made for an lld link.
///
/// GNU ld 2.29.1 (`arm-none-symbianelf`) reaches an imported function through an 8-byte
/// PLT entry, `ldr pc, [pc, #-4]` and a word that elf2e32 turns into the import. lld has no
/// such PLT for ARM: its entries take 16 bytes plus a 4-byte `.got.plt` slot, and 32 + 12
/// fixed (experiment 109). For each imported function the program calls, this object
/// defines `__wrap_<f>` as GNU's entry, its word an `R_ARM_ABS32` against `__real_<f>`;
/// linked with `--wrap=<f>` for every one, the calls reach the stubs, the words become
/// `R_ARM_ABS32` imports of `<f>`, and lld makes no PLT.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportStubs {
    functions: Vec<String>,
}

impl ImportStubs {
    /// The input section that holds the stubs (`.text.*`: placed with the code).
    pub const SECTION: &str = ".text.symdev_import_stubs";

    /// Stubs for these imported functions, sorted, each once.
    pub fn new(functions: impl IntoIterator<Item = String>) -> Result<Self> {
        let functions: std::collections::BTreeSet<String> = functions.into_iter().collect();
        if let Some(bad) = functions.iter().find(|f| f.is_empty() || f.contains('\0')) {
            return Err(Error::Other(format!(
                "import stub for {bad:?}: a symbol name is non-empty and has no NUL byte"
            )));
        }
        Ok(Self {
            functions: functions.into_iter().collect(),
        })
    }

    /// Stubs for the imported functions a first lld link of the same inputs calls through
    /// its PLT (`R_ARM_JUMP_SLOT`). Imports it only takes the address of (`R_ARM_ABS32`)
    /// need none, as with GNU ld.
    pub fn from_first_link(elf: &ElfImage) -> Result<Self> {
        if elf.linker() != ElfLinker::Lld {
            return Err(Error::Other(
                "import stubs are read from a first link by lld (rust-lld -flavor gnu); this \
                 ELF was not linked by lld"
                    .into(),
            ));
        }
        Self::new(elf.plt_imports()?)
    }

    /// The imported functions, sorted: the link needs `--wrap=<f>` for each.
    pub fn functions(&self) -> &[String] {
        &self.functions
    }
}

#[cfg(test)]
mod tests;
