//! `GcceBuild`: compiler argv (`arm-none-symbianelf-g++`).
use std::path::{Path, PathBuf};

use super::module::Module;
use super::{GcceBuild, arg};

/// Extra `-I` directories: `user` after the source directory (build dir for `.rsg`,
/// `USERINCLUDE`), `system` after `epoc32/include` (`SYSTEMINCLUDE`, case-fold overlay).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CompileIncludes {
    pub user: Vec<PathBuf>,
    pub system: Vec<PathBuf>,
}

impl GcceBuild {
    pub fn compile_args(
        &self,
        source_dir: &Path,
        includes: &CompileIncludes,
        source: &Path,
        obj: &Path,
    ) -> Vec<String> {
        self.compile_args_for(&self.exe_module(), source_dir, includes, source, obj)
    }

    /// `-D__EXE__` or `-D__DLL__` by module (SDK `cl_bpabi.pm`).
    pub fn compile_args_for(
        &self,
        module: &Module,
        source_dir: &Path,
        includes: &CompileIncludes,
        source: &Path,
        obj: &Path,
    ) -> Vec<String> {
        let epoc = &self.tools.epocroot;
        let mut args = vec![
            arg(&self.tools.gxx),
            "-O2".into(),
            "-fexceptions".into(),
            "-march=armv5t".into(),
            "-mapcs".into(),
            "-mthumb-interwork".into(),
            "-mthumb".into(),
            "-msoft-float".into(),
            // SDK headers predate GCC 12 (extra member qualification, narrowing UIDs):
            // accept them as older compilers did (experiment 51).
            "-fpermissive".into(),
            "-Wno-narrowing".into(),
            "-D__SYMBIAN32__".into(),
            "-D__EPOC32__".into(),
            "-D__MARM__".into(),
            "-D__GCCE__".into(),
            if module.dll { "-D__DLL__" } else { "-D__EXE__" }.into(),
            "-include".into(),
            arg(&epoc.join("epoc32/include/gcce/gcce.h")),
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
            "-D__SUPPORT_CPP_EXCEPTIONS__".into(),
            "-I".into(),
            arg(source_dir),
        ];
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
        args
    }
}
