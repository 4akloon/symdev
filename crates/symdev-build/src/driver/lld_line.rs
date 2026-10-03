//! `LldLine`: the recorded link line made into rust-lld's (experiments 109, 112).
use std::path::{Path, PathBuf};

use symdev_core::{Error, Result};

use super::arg;
use crate::sdk_lld_copy::SdkLldCopy;

/// What turns the recorded line, written for rust-lld ([`super::Linker::lld`]), into the
/// line rust-lld links with: experiment 109 §2's changes, each forced by an lld error or a
/// wrong result there.
///
/// 1. `--default-symver` goes: lld does not know it, and an EXE exports nothing.
/// 2. The SDK's `lib` and `urel` become the fixed copies ([`crate::SdkLldCache`]): lld
///    refuses the `.dso` string-table padding, and needs `usrt2_2.lib`'s `R_ARM_TARGET2`
///    as `R_ARM_ABS32`.
/// 3. `-z notext`: GCCE, RVCT and rustc code here is not PIC (GNU writes `DT_TEXTREL` too).
/// 4. `--target2=abs`: `R_ARM_TARGET2` is absolute on Symbian, as GNU's symbianelf has it.
/// 5. `-Bsymbolic`: the image's own functions are called directly, not through a PLT
///    slot elf2e32 cannot read.
/// 6. `-T symbian-lld.ld`: the layout and the symbols `eexe.lib` needs.
/// 7. With the prebuilt Avkon shim, `--defsym=symrs_uid3=0x<uid3>`: the shim reads the
///    application's UID3 from that symbol's address.
///
/// The options go at the end of the line, where experiments 109 and 112 put them; the
/// order of the inputs, which decides the layout, is the recorded line's.
pub struct LldLine {
    /// The SDK's `epoc32/release/armv5/lib` and `urel`, as the recorded line names them.
    pub sdk_lib: PathBuf,
    pub sdk_urel: PathBuf,
    /// The fixed copies that replace them.
    pub copy: SdkLldCopy,
    /// `symbian-rs/targets/symbian-lld.ld` ([`crate::RustSdk::lld_script`]).
    pub script: PathBuf,
    /// `Some(uid3)` when the Avkon shim is the prebuilt one.
    pub uid3_symbol: Option<u32>,
}

impl LldLine {
    /// The line rust-lld links with. Refuses a line that does not name both of the SDK's
    /// directories, which the fixed copies must replace.
    pub fn adapt(&self, line: Vec<String>) -> Result<Vec<String>> {
        let (lib, urel) = (Self::dir(&self.sdk_lib), Self::dir(&self.sdk_urel));
        for dir in [&lib, &urel] {
            if !line.contains(dir) {
                return Err(Error::Other(format!(
                    "the link line names no {dir}, so rust-lld would read the SDK's own \
                     files there instead of the fixed copies"
                )));
            }
        }
        let mut out: Vec<String> = line
            .into_iter()
            .filter(|a| a != "--default-symver")
            .map(|a| match a {
                a if a == lib => Self::dir(&self.copy.lib()),
                a if a == urel => Self::dir(&self.copy.urel()),
                a => a,
            })
            .collect();
        out.extend(["-z", "notext", "--target2=abs", "-Bsymbolic", "-T"].map(String::from));
        out.push(arg(&self.script));
        if let Some(uid3) = self.uid3_symbol {
            out.push(format!("--defsym=symrs_uid3=0x{uid3:08x}"));
        }
        Ok(out)
    }

    /// The files the line's `-l:` arguments name, each once, in order: what
    /// [`crate::SdkLldCache::ensure`] copies.
    pub fn sdk_files(line: &[String]) -> Vec<String> {
        let mut files: Vec<String> = Vec::new();
        for name in line.iter().filter_map(|a| a.strip_prefix("-l:")) {
            if !files.iter().any(|f| f == name) {
                files.push(name.to_string());
            }
        }
        files
    }

    /// The second link of experiment 112 §3: `first` writing `elf` instead of its own
    /// output, with `stubs` (the [`symdev_elf2e32::ImportStubs`] object) after every other
    /// input and `--wrap=<f>` for each function it stubs.
    pub fn second_link(
        first: &[String],
        elf: &Path,
        stubs: &Path,
        functions: &[String],
    ) -> Result<Vec<String>> {
        let mut out = first.to_vec();
        let output = out
            .iter()
            .position(|a| a == "-o")
            .and_then(|at| out.get_mut(at + 1))
            .ok_or_else(|| Error::Other("the first rust-lld link has no -o to redirect".into()))?;
        *output = arg(elf);
        out.push(arg(stubs));
        out.extend(functions.iter().map(|f| format!("--wrap={f}")));
        Ok(out)
    }

    fn dir(path: &Path) -> String {
        format!("-L{}", path.display())
    }
}
