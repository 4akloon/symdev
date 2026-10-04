//! `RustBuild`: the rust-lld link — two links around the import stubs (experiment 113).
use std::path::{Path, PathBuf};
use std::process::Command;

use symdev_core::{Error, RemotePath, Result};
use symdev_elf2e32::{ElfImage, ImportStubs};

use super::{LinkInputs, Linker, LldLine, RustBuild, arg};
use crate::{RustLld, RustPrebuilt, SdkLldCache, file_error};

impl RustBuild {
    /// Links `build/<name>.elf` with rust-lld, as experiment 112 §8 lays out: the first
    /// link writes `<name>.first.elf`; the imported functions it calls through lld's PLT get
    /// GNU ld's 8-byte stubs in `build/import_stubs.o`, and the second link — the same line
    /// writing `<name>.elf`, with the stubs and a `--wrap` per function — has no PLT left,
    /// which is checked. With no such function the first link is the result. Both links
    /// write the same map, so it describes the ELF that is post-linked.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn link_lld(
        &self,
        rust_lld: &Path,
        cache: &SdkLldCache,
        prebuilt: Option<&RustPrebuilt>,
        inputs: &LinkInputs,
        elf: &Path,
        map: &Path,
        cwd: &RemotePath,
    ) -> Result<()> {
        let lld = rust_lld;
        let linker = match prebuilt {
            Some(p) => Linker::lld(lld, p.lib_dir().into(), p.lib_dir().into()),
            None => {
                let g = self.gcce.tools.gcce()?;
                Linker::lld(lld, g.gcc_lib.clone(), g.gcc_target_lib.clone())
            }
        };
        let first_elf = elf.with_extension("first.elf");
        let line = self.link_line(
            &linker,
            inputs.rust,
            inputs.shims,
            Some(inputs.libcalls),
            &first_elf,
            map,
        )?;
        let armv5 = self.gcce.tools.epocroot.join("epoc32/release/armv5");
        let lld_line = LldLine {
            sdk_lib: armv5.join("lib"),
            sdk_urel: armv5.join("urel"),
            copy: cache.ensure(&self.gcce.tools.epocroot, &LldLine::sdk_files(&line))?,
            script: self.sdk.lld_script()?,
            uid3_symbol: (prebuilt.is_some() && self.ui.is_some()).then_some(self.gcce.uid3),
        };
        let first = lld_line.adapt(line)?;
        self.run_link("first", &first, cwd)?;
        let stubs = ImportStubs::from_first_link(&Self::elf_at(&first_elf)?)
            .map_err(|e| file_error(&first_elf, e))?;
        if stubs.functions().is_empty() {
            std::fs::rename(&first_elf, elf).map_err(|e| file_error(elf, e))?;
        } else {
            let object = elf.with_file_name("import_stubs.o");
            std::fs::write(&object, stubs.object()).map_err(|e| file_error(&object, e))?;
            let second = LldLine::second_link(&first, elf, &object, stubs.functions())?;
            self.run_link("second", &second, cwd)?;
        }
        let left = Self::elf_at(elf)?.jump_slots()?;
        if !left.is_empty() {
            return Err(Error::Other(format!(
                "{} still calls {} through lld's PLT, which elf2e32 cannot turn into \
                 imports: the import stubs did not cover them; set SYMDEV_RUST_LINKER=gnu to \
                 link with GCCE's GNU ld, and report it",
                elf.display(),
                left.join(", ")
            )));
        }
        Ok(())
    }

    /// One rust-lld run; its error says which of the two links failed and the way back.
    fn run_link(&self, which: &str, args: &[String], cwd: &RemotePath) -> Result<()> {
        self.gcce.run_tool(args, cwd).map_err(|e| {
            Error::Other(format!(
                "the {which} rust-lld link failed: {e}; set SYMDEV_RUST_LINKER=gnu to link \
                 with GCCE's GNU ld instead"
            ))
        })
    }

    fn elf_at(path: &Path) -> Result<ElfImage> {
        let bytes = std::fs::read(path).map_err(|e| file_error(path, e))?;
        ElfImage::parse(bytes).map_err(|e| file_error(path, e))
    }

    /// What a rust-lld link needs before cargo runs: the SDK's linker script, and rust-lld
    /// itself — `set` (`SYMDEV_RUST_LLD`), else the project's toolchain's.
    pub(super) fn rust_lld_ready(&self, set: Option<&Path>, cwd: &RemotePath) -> Result<PathBuf> {
        self.sdk.lld_script()?;
        match set {
            Some(set) => Ok(set.to_path_buf()),
            None => self.rust_lld(cwd),
        }
    }

    /// The rust-lld of the project's toolchain: `rustc` run in the project, so that its
    /// `rust-toolchain.toml` picks the SDK's nightly.
    fn rust_lld(&self, cwd: &RemotePath) -> Result<PathBuf> {
        let sysroot = self.rustc_says(&["--print", "sysroot"], cwd)?;
        let verbose = self.rustc_says(&["-vV"], cwd)?;
        let lld = RustLld::in_sysroot(&sysroot, &verbose)?;
        if !lld.path().is_file() {
            return Err(Error::Other(format!(
                "the project's Rust toolchain has no rust-lld at {} (it comes with rustup's \
                 rustc component); set {} to a rust-lld, or SYMDEV_RUST_LINKER=gnu to link \
                 with GCCE's GNU ld",
                lld.path().display(),
                RustLld::VARIABLE
            )));
        }
        Ok(lld.path().to_path_buf())
    }

    fn rustc_says(&self, args: &[&str], cwd: &RemotePath) -> Result<String> {
        let mut cmd = Command::new(&self.rustc);
        cmd.args(args);
        // As for cargo: a proxy's RUSTUP_TOOLCHAIN would override the project's nightly.
        cmd.env_remove("RUSTUP_TOOLCHAIN");
        let shown = format!("{} {}", arg(&self.rustc), args.join(" "));
        let out = self
            .gcce
            .env
            .run_blocking(cmd, cwd)
            .map_err(|e| Error::Other(format!("{shown}: {e}")))?;
        if out.status != 0 {
            return Err(Error::Other(format!(
                "{shown} failed (status {}): {}",
                out.status,
                String::from_utf8_lossy(&out.stderr)
            )));
        }
        Ok(String::from_utf8_lossy(&out.stdout).into_owned())
    }
}
