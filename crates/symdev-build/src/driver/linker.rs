//! `Linker`: the program a link line runs and where it finds the GCC runtime.
use std::path::{Path, PathBuf};

use super::arg;
use crate::toolchain::GcceTools;

/// What the recorded link line takes from the toolchain: the program, and the two
/// directories `-lsupc++ -lgcc` are searched in (`-L<gcc_lib>/ -L <gcc_target_lib>` at the
/// front, `-L<gcc_target_lib>` again before the runtime DSOs). Everything else on the line
/// is the SDK's or the project's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Linker {
    /// argv up to the first option: `ld`, or `rust-lld -flavor gnu`.
    pub program: Vec<String>,
    pub gcc_lib: PathBuf,
    pub gcc_target_lib: PathBuf,
}

impl Linker {
    /// GCCE's GNU ld 2.29.1 and its runtime: the line recorded in experiment 5.
    pub fn gnu(gcce: &GcceTools) -> Self {
        Self {
            program: vec![arg(&gcce.ld)],
            gcc_lib: gcce.gcc_lib.clone(),
            gcc_target_lib: gcce.gcc_target_lib.clone(),
        }
    }

    /// rust-lld in its GNU flavour (experiments 109, 112), with the GCC runtime in
    /// `gcc_lib` and `gcc_target_lib`: GCCE's two directories, or the Rust SDK's prebuilt
    /// `lib/` for both.
    pub fn lld(rust_lld: &Path, gcc_lib: PathBuf, gcc_target_lib: PathBuf) -> Self {
        Self {
            program: vec![arg(rust_lld), "-flavor".into(), "gnu".into()],
            gcc_lib,
            gcc_target_lib,
        }
    }
}
