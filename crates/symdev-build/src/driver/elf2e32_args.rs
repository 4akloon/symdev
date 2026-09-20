//! `GcceBuild`: elf2e32 argv and invocation.
use std::path::Path;

use symdev_core::{RemotePath, Result};

use super::module::Module;
use super::{GcceBuild, arg};

impl GcceBuild {
    pub fn elf2e32_args(&self, name: &str, elf: &Path, exe: &Path) -> Vec<String> {
        self.elf2e32_args_for(&self.exe_module(), name, elf, exe, None, None)
    }

    /// EXE: the recorded experiment-6 argv. DLL: the SDK recipe (`--sid`, UID1
    /// `0x10000079`, `--uid2`, `--targettype=DLL`, `--definput=<frozen .def>` when it
    /// exists else `--ignorenoncallable`, `--dlldata` for `EPOCALLOWDLLDATA`, `--dso` and
    /// `--defoutput` next to the output). `build_dir` joins `--libpath` (a `;` list per
    /// `elf2e32 --help`) so this project's own `.dso` files resolve.
    pub fn elf2e32_args_for(
        &self,
        module: &Module,
        name: &str,
        elf: &Path,
        out: &Path,
        build_dir: Option<&Path>,
        frozen_def: Option<&Path>,
    ) -> Vec<String> {
        let argv0 = match &self.tools.elf2e32 {
            Some(tool) => arg(tool),
            None => "elf2e32".into(),
        };
        let mut args = vec![argv0];
        if module.dll {
            args.extend([
                format!("--sid=0x{:08x}", module.uid3),
                "--uid1=0x10000079".into(),
                format!("--uid2=0x{:08x}", module.uid2),
            ]);
        } else {
            args.push("--uid1=0x1000007a".into());
        }
        args.push(format!("--uid3=0x{:08x}", module.uid3));
        // Empty `--capability=` is rejected: "Option capability has missed argument"
        // (elf2e32_next 3.0 Build 2). Omit the flag when the manifest list is empty.
        if !self.capabilities.is_empty() {
            args.push(format!("--capability={}", self.capabilities.join("+")));
        }
        args.push("--fpu=softvfp".into());
        if module.dll {
            let dir = out.parent().unwrap_or(Path::new("."));
            args.push("--targettype=DLL".into());
            args.push(match frozen_def {
                Some(def) => format!("--definput={}", def.display()),
                None => "--ignorenoncallable".into(),
            });
            if module.allow_data {
                args.push("--dlldata".into());
            }
            args.extend([
                format!("--output={}", out.display()),
                format!("--dso={}", dir.join(format!("{name}.dso")).display()),
                format!("--defoutput={}", dir.join(format!("{name}.def")).display()),
            ]);
        } else {
            args.extend([
                "--targettype=EXE".into(),
                format!("--output={}", out.display()),
            ]);
        }
        let sdk_lib = self.tools.epocroot.join("epoc32/release/armv5/lib");
        let libpath = match build_dir {
            Some(dir) => format!("{};{}", sdk_lib.display(), dir.display()),
            None => sdk_lib.display().to_string(),
        };
        args.extend([
            format!("--elfinput={}", elf.display()),
            format!("--linkas={}", Self::linkas_for(module, name)),
            format!("--libpath={libpath}"),
        ]);
        args
    }

    /// External `SYMDEV_ELF2E32` when set; otherwise native `symdev-elf2e32` on the
    /// same argv.
    pub(super) fn run_elf2e32(&self, args: &[String], cwd: &RemotePath) -> Result<()> {
        if self.tools.elf2e32.is_some() {
            return self.run_tool(args, cwd);
        }
        symdev_elf2e32::Elf2E32::from_args(args)?
            .write_outputs()
            .map(|_| ())
    }
}
