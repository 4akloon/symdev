//! `GcceBuild`: compiler argv (`arm-none-symbianelf-g++`).
use std::path::{Path, PathBuf};

use symdev_core::Result;

use super::language::SourceLanguage;
use super::module::Module;
use super::{GcceBuild, arg};
use crate::Mmp;

/// Extra `-I` directories: `user` after the source directory (build dir for `.rsg`,
/// `USERINCLUDE`), `system` after `epoc32/include` (`SYSTEMINCLUDE`, case-fold overlay).
/// `prefix` holds extra `-include` headers, force-included after the SDK's `gcce.h`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CompileIncludes {
    pub user: Vec<PathBuf>,
    pub system: Vec<PathBuf>,
    pub prefix: Vec<PathBuf>,
}

/// What the `.mmp` adds to the compiler command line beside its includes: the `MACRO`
/// arguments and the `OPTION GCCE` text (mmp-frontend-spec.md §8.2 item 6 and §8.4
/// position 12).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CompileFlags {
    pub macros: Vec<String>,
    pub option: Vec<String>,
}

impl CompileFlags {
    pub fn of(mmp: &Mmp) -> Self {
        Self {
            macros: mmp.macros.clone(),
            option: mmp
                .option("GCCE")
                .unwrap_or_default()
                .split_whitespace()
                .map(str::to_string)
                .collect(),
        }
    }
}

impl GcceBuild {
    pub fn compile_args(
        &self,
        source_dir: &Path,
        includes: &CompileIncludes,
        source: &Path,
        obj: &Path,
    ) -> Result<Vec<String>> {
        self.compile_args_for(
            &self.exe_module(),
            &CompileFlags::default(),
            source_dir,
            includes,
            source,
            obj,
        )
    }

    /// `-D__EXE__` or `-D__DLL__` by module (SDK `cl_bpabi.pm`).
    pub fn compile_args_for(
        &self,
        module: &Module,
        flags: &CompileFlags,
        source_dir: &Path,
        includes: &CompileIncludes,
        source: &Path,
        obj: &Path,
    ) -> Result<Vec<String>> {
        let epoc = &self.tools.epocroot;
        let language = SourceLanguage::of(source)?;
        let mut args = vec![
            arg(&self.tools.gxx),
            "-O2".into(),
            "-fexceptions".into(),
            "-march=armv5t".into(),
            "-mapcs".into(),
        ];
        // §8.4 position 12: after the architecture flags, before the instruction set and
        // the floating-point option, so those still win over an `OPTION GCCE` that
        // contradicts them.
        args.extend(flags.option.iter().cloned());
        args.extend([
            "-mthumb-interwork".into(),
            "-mthumb".into(),
            "-msoft-float".into(),
        ]);
        // C++ keeps the leniency flags the GCC-12-era SDK headers need (experiment 51);
        // `.c` goes through the C front end instead (experiment 59).
        args.extend(language.args());
        args.extend([
            "-D__SYMBIAN32__".into(),
            "-D__EPOC32__".into(),
            "-D__MARM__".into(),
            "-D__GCCE__".into(),
            if module.dll { "-D__DLL__" } else { "-D__EXE__" }.into(),
            "-include".into(),
            arg(&epoc.join("epoc32/include/gcce/gcce.h")),
        ]);
        // After `gcce.h`, so a generated header can repair what it defines (`GcceCompat`).
        for header in &includes.prefix {
            args.extend(["-include".into(), arg(header)]);
        }
        args.extend([
            format!(
                "-D__PRODUCT_INCLUDE__=\"{}\"",
                epoc.join("epoc32/include/variant/symbian_os_v9.3.hrh")
                    .display()
            ),
            "-nostdinc".into(),
            "-c".into(),
            "-D__MARM_THUMB__".into(),
            "-D__MARM_INTERWORK__".into(),
            "-DNDEBUG".into(),
            "-D_UNICODE".into(),
            "-D__S60_3X__".into(),
            "-D__SERIES60_3X__".into(),
            "-D__EABI__".into(),
            "-D__MARM_ARMV5__".into(),
        ]);
        // §8.2 item 6: the `.mmp`'s own macros, verbatim and in source order.
        args.extend(flags.macros.iter().map(|m| format!("-D{m}")));
        args.extend([
            "-D__SUPPORT_CPP_EXCEPTIONS__".into(),
            "-I".into(),
            arg(source_dir),
        ]);
        for dir in &includes.user {
            args.extend(["-I".into(), arg(dir)]);
        }
        args.extend([
            "-I".into(),
            arg(&epoc.join("epoc32/include")),
            "-I".into(),
            arg(&epoc.join("epoc32/include/variant")),
        ]);
        for dir in &includes.system {
            args.extend(["-I".into(), arg(dir)]);
        }
        args.extend([
            "-I".into(),
            arg(&self.tools.gcc_lib.join("include")),
            "-o".into(),
            arg(obj),
            arg(source),
        ]);
        Ok(args)
    }
}
