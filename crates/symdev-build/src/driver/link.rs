//! `GcceBuild`: linker argv (`arm-none-symbianelf-ld`).
use std::path::{Path, PathBuf};

use super::module::Module;
use super::{GcceBuild, arg};

impl GcceBuild {
    /// Recorded experiment-5 link; `libraries` (MMP `LIBRARY`, as `.dso`) are added
    /// after the recorded runtime DSOs.
    pub fn link_args(
        &self,
        name: &str,
        obj: &Path,
        elf: &Path,
        map: &Path,
        libraries: &[String],
    ) -> Vec<String> {
        self.link_args_for(&self.exe_module(), name, obj, elf, map, libraries, &[])
    }

    /// DLLs link `edll.lib` with entry `_E32Dll` (SDK `cl_bpabi.pm`); `lib_dirs` are
    /// searched for this project's own `.dso` files.
    #[allow(clippy::too_many_arguments)]
    pub fn link_args_for(
        &self,
        module: &Module,
        name: &str,
        obj: &Path,
        elf: &Path,
        map: &Path,
        libraries: &[String],
        lib_dirs: &[PathBuf],
    ) -> Vec<String> {
        let (entry, first_lib) = if module.dll {
            ("_E32Dll", "-l:edll.lib")
        } else {
            ("_E32Startup", "-l:eexe.lib")
        };
        let mut args: Vec<String> = vec![
            arg(&self.tools.ld),
            format!("-L{}/", self.tools.gcc_lib.display()),
            "-L".into(),
            arg(&self.tools.gcc_target_lib),
            "--target1-abs".into(),
            "--no-undefined".into(),
            "-nostdlib".into(),
            "-shared".into(),
            "-Ttext".into(),
            "0x8000".into(),
            "-Tdata".into(),
            "0x400000".into(),
            "--default-symver".into(),
            "-soname".into(),
            Self::linkas_for(module, name),
            "--target1-abs".into(),
            "--no-undefined".into(),
            "-nostdlib".into(),
            "--strip-debug".into(),
            "--entry".into(),
            entry.into(),
            "-u".into(),
            entry.into(),
            format!(
                "-L{}",
                self.tools
                    .epocroot
                    .join("epoc32/release/armv5/urel")
                    .display()
            ),
            first_lib.into(),
            "-o".into(),
            arg(elf),
            "-Map".into(),
            arg(map),
            arg(obj),
            "-(".into(),
            "-l:usrt2_2.lib".into(),
            "-)".into(),
            format!("-L{}", self.tools.gcc_target_lib.display()),
            format!(
                "-L{}",
                self.tools
                    .epocroot
                    .join("epoc32/release/armv5/lib")
                    .display()
            ),
            "-l:euser.dso".into(),
            "-l:dfpaeabi.dso".into(),
            "-l:dfprvct2_2.dso".into(),
            "-l:drtaeabi.dso".into(),
            "-l:scppnwdl.dso".into(),
            "-l:drtrvct2_2.dso".into(),
        ];
        let at = args.len() - 6;
        for (i, dir) in lib_dirs.iter().enumerate() {
            args.insert(at + i, format!("-L{}", dir.display()));
        }
        for lib in libraries {
            let flag = format!("-l:{lib}");
            if !args.contains(&flag) {
                args.push(flag);
            }
        }
        args.extend(["-lsupc++".into(), "-lgcc".into()]);
        args
    }
}
