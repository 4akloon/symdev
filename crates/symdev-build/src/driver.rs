use std::path::{Path, PathBuf};
use std::process::Command;

use symdev_core::{Artifact, BuildBackend, Error, LocalEnv, Project, RemotePath, Result};

use crate::icons::{AppIcon, MifConvTool, MifFile};
use crate::resources::{ProjectMmps, SdkIncludeCaseFold};
use crate::toolchain::Toolchain;
use crate::{Mmp, MmpResource};

/// What one MMP builds: its E32 kind and UIDs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Module {
    pub dll: bool,
    pub uid2: u32,
    pub uid3: u32,
    /// `EPOCALLOWDLLDATA`: writable static data in a DLL (`--dlldata`).
    pub allow_data: bool,
}

impl Module {
    /// DLL UIDs come from the MMP `UID <uid2> <uid3>` line; EXEs keep the recorded
    /// experiment-6 UIDs (UID2 omitted, UID3 from the manifest).
    pub fn of(mmp: &Mmp, manifest_uid3: u32) -> Result<Self> {
        if mmp.is_dll() {
            let [uid2, uid3] = mmp.uid[..] else {
                return Err(Error::Other(format!(
                    "{}: TARGETTYPE DLL needs `UID <uid2> <uid3>`",
                    mmp.target
                )));
            };
            return Ok(Self {
                dll: true,
                uid2,
                uid3,
                allow_data: mmp.epocallowdlldata,
            });
        }
        Ok(Self {
            dll: false,
            uid2: 0,
            uid3: manifest_uid3,
            allow_data: false,
        })
    }

    fn ext(&self) -> &'static str {
        if self.dll { "dll" } else { "exe" }
    }
}

/// Extra `-I` directories: `user` after the source directory (build dir for `.rsg`,
/// `USERINCLUDE`), `system` after `epoc32/include` (`SYSTEMINCLUDE`, case-fold overlay).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CompileIncludes {
    pub user: Vec<PathBuf>,
    pub system: Vec<PathBuf>,
}

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

impl GcceBuild {
    fn linkas_for(module: &Module, name: &str) -> String {
        format!("{name}{{000a0000}}[{:08x}].{}", module.uid3, module.ext())
    }

    fn exe_module(&self) -> Module {
        Module {
            dll: false,
            uid2: 0,
            uid3: self.uid3,
            allow_data: false,
        }
    }

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

    /// `.rss` → `.rsc` (+ `.rsg` with `HEADER`) with the SDK `cpp.exe` and `rcomp.exe`
    /// under Wine (experiment 9 argv, Wine `Z:` paths).
    /// `<app>_aif.mif` from the SVG via Wine `mifconv` (experiment 55). It runs in
    /// `build/mifconv-temp` on a copy named `<app>.svg` with relative paths: the input
    /// path becomes part of its temporary `.svgb` name, and with a long one
    /// `svgtbinencode` silently writes nothing (a MIF with empty icons).
    fn compile_icon(&self, icon: &AppIcon, build_dir: &Path) -> Result<()> {
        if !icon.source.is_file() {
            return Err(Error::Other(format!(
                "icon not found: {}",
                icon.source.display()
            )));
        }
        let tools = self.tools.epocroot.join("epoc32/tools");
        let work = build_dir.join("mifconv-temp");
        if work.exists() {
            std::fs::remove_dir_all(&work).map_err(io)?;
        }
        std::fs::create_dir_all(work.join("t")).map_err(io)?;
        let svg = format!("{}.svg", icon.app);
        std::fs::copy(&icon.source, work.join(&svg)).map_err(io)?;
        let mif = format!("{}_aif.mif", icon.app);
        let mbg = format!("{}_aif.mbg", icon.app);
        let tool = MifConvTool {
            wine: self.tools.wine.clone(),
            mifconv: tools.join("mifconv.exe"),
            encoder_dir: tools,
        };
        self.run_tool_env(
            &tool.args(&mif, &mbg, "t", &svg),
            &RemotePath::new(arg(&work)),
            &[("WINEDEBUG", "-all".into())],
        )?;
        // mifconv exits 0 after printing its usage on bad input: check what it wrote.
        let bytes = std::fs::read(work.join(&mif))
            .map_err(|e| Error::Other(format!("mifconv wrote no {mif}: {e}")))?;
        MifFile::check(&bytes).map_err(|e| Error::Other(format!("{mif}: {e}")))?;
        std::fs::rename(work.join(&mif), icon.mif(build_dir)).map_err(io)?;
        std::fs::rename(work.join(&mbg), icon.mbg(build_dir)).map_err(io)?;
        Ok(())
    }

    fn compile_resource(
        &self,
        res: &MmpResource,
        mmp_dir: &Path,
        mmp: &Mmp,
        build_dir: &Path,
        cwd: &RemotePath,
    ) -> Result<()> {
        use symdev_rcomp::{RcompTool, RssCppTool};
        let rss = res.source(mmp_dir);
        if !rss.is_file() {
            return Err(Error::Other(format!(
                "resource not found: {}",
                rss.display()
            )));
        }
        let stem = res.stem()?;
        let rpp = build_dir.join(format!("{stem}.rpp"));
        let rsc = build_dir.join(format!("{stem}.rsc"));
        let rsg = build_dir.join(format!("{stem}.rsg"));
        let epoc = self.tools.epocroot.join("epoc32");
        let mut includes = vec![
            rss.parent().unwrap_or(mmp_dir).to_path_buf(),
            build_dir.to_path_buf(),
        ];
        includes.extend(
            mmp.userinclude
                .iter()
                .map(|d| Self::mmp_dir_path(mmp_dir, d)),
        );
        includes.push(epoc.join("include"));
        includes.extend(
            mmp.systeminclude
                .iter()
                .map(|d| Self::mmp_dir_path(mmp_dir, d)),
        );
        let wine_env = [
            ("WINEPATH", arg(&epoc.join("tools"))),
            ("WINEDEBUG", "-all".into()),
        ];
        let cpp = RssCppTool::new(&self.tools.wine, &epoc.join("gcc/bin/cpp.exe"));
        self.run_tool_env(
            &cpp.args(
                &includes,
                &RssCppTool::wine_path(&rss),
                &RssCppTool::wine_path(&rpp),
            ),
            cwd,
            &wine_env,
        )?;
        let rcomp = RcompTool::new(&self.tools.wine, &epoc.join("tools/rcomp.exe"));
        let (o, s, i) = (
            RssCppTool::wine_path(&rsc),
            RssCppTool::wine_path(&rpp),
            RssCppTool::wine_path(&rss),
        );
        let args = if res.header {
            rcomp.args_with_header(&o, &RssCppTool::wine_path(&rsg), &s, &i)
        } else {
            rcomp.args(&o, &s, &i)
        };
        self.run_tool_env(&args, cwd, &wine_env)?;
        if !rsc.is_file() {
            return Err(Error::Other(format!("rcomp wrote no {}", rsc.display())));
        }
        Ok(())
    }

    /// MMP include directory (`..\\inc` style) relative to the MMP.
    fn mmp_dir_path(mmp_dir: &Path, dir: &str) -> PathBuf {
        let dir = dir.replace('\\', "/");
        if Path::new(&dir).is_absolute() {
            PathBuf::from(dir)
        } else {
            mmp_dir.join(dir)
        }
    }

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
    fn run_elf2e32(&self, args: &[String], cwd: &RemotePath) -> Result<()> {
        if self.tools.elf2e32.is_some() {
            return self.run_tool(args, cwd);
        }
        symdev_elf2e32::Elf2E32::from_args(args)?
            .write_outputs()
            .map(|_| ())
    }

    fn run_tool(&self, args: &[String], cwd: &RemotePath) -> Result<()> {
        self.run_tool_env(args, cwd, &[])
    }

    fn run_tool_env(
        &self,
        args: &[String],
        cwd: &RemotePath,
        env: &[(&str, String)],
    ) -> Result<()> {
        let mut cmd = Command::new(&args[0]);
        cmd.args(&args[1..]);
        for (k, v) in env {
            cmd.env(k, v);
        }
        let out = self.env.run_blocking(cmd, cwd)?;
        if out.status != 0 {
            return Err(Error::Other(format!(
                "{} failed (status {}): {}",
                args[0],
                out.status,
                String::from_utf8_lossy(&out.stderr)
            )));
        }
        Ok(())
    }
}

// Source lookup dialect (first existing file): last SOURCEPATH/SOURCE, then
// MMP-directory/SOURCE, then project-root/SOURCE.
fn resolve_source(
    sourcepath: Option<&str>,
    mmp_dir: &Path,
    project_root: &Path,
    source: &str,
) -> Result<PathBuf> {
    // MMP paths use `\\` (Symbian); the host uses `/`.
    let source = source.replace('\\', "/");
    let mut candidates = Vec::new();
    if let Some(sp) = sourcepath {
        let sp = PathBuf::from(sp.replace('\\', "/"));
        if sp.is_absolute() {
            candidates.push(sp.join(&source));
        } else {
            candidates.push(mmp_dir.join(sp).join(&source));
        }
    }
    candidates.push(mmp_dir.join(&source));
    candidates.push(project_root.join(&source));
    candidates
        .into_iter()
        .find(|p| p.is_file())
        .ok_or_else(|| Error::Other(format!("source not found: {source}")))
}

fn io(err: std::io::Error) -> Error {
    Error::Other(err.to_string())
}

impl BuildBackend for GcceBuild {
    fn build(&self, project: &Project) -> Result<Vec<Artifact>> {
        let mmps = ProjectMmps::load(project)?;
        let build_dir = project.root.join("build");
        std::fs::create_dir_all(&build_dir).map_err(io)?;
        let cwd = RemotePath::new(arg(&project.root));
        let casefold = SdkIncludeCaseFold::ensure(
            &self.tools.epocroot.join("epoc32/include"),
            &build_dir.join("sdk-include-casefold"),
        )?;

        let mut artifacts = Vec::new();
        if let Some(source) = &self.icon {
            let icon = AppIcon::of(project, source)?;
            self.compile_icon(&icon, &build_dir)?;
            artifacts.push(icon.artifact(&build_dir));
        }
        for (mmp_dir, mmp) in &mmps.mmps {
            let name = mmp.name();
            let module = Module::of(mmp, self.uid3)?;
            // Resources first: sources include the generated `.rsg` headers.
            for res in &mmp.resource {
                self.compile_resource(res, mmp_dir, mmp, &build_dir, &cwd)?;
            }
            let mut includes = CompileIncludes {
                user: vec![build_dir.clone()],
                system: Vec::new(),
            };
            includes.user.extend(
                mmp.userinclude
                    .iter()
                    .map(|d| Self::mmp_dir_path(mmp_dir, d)),
            );
            includes.system.extend(
                mmp.systeminclude
                    .iter()
                    .map(|d| Self::mmp_dir_path(mmp_dir, d)),
            );
            includes.system.push(casefold.clone());

            let mut objs = Vec::new();
            for (i, src) in mmp.source.iter().enumerate() {
                let sp = mmp.source_sourcepath.get(i).cloned().flatten();
                let source = resolve_source(sp.as_deref(), mmp_dir, &project.root, src)?;
                let source_dir = source.parent().unwrap_or(mmp_dir);
                let stem = source
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or(src.as_str());
                let obj = build_dir.join(format!("{stem}.o"));
                self.run_tool(
                    &self.compile_args_for(&module, source_dir, &includes, &source, &obj),
                    &cwd,
                )?;
                objs.push(obj);
            }
            let obj = objs
                .first()
                .ok_or_else(|| Error::Other("no SOURCE".into()))?;
            let elf = build_dir.join(format!("{name}.elf"));
            let map = build_dir.join(format!("{name}.{}.map", module.ext()));
            let mut link = self.link_args_for(
                &module,
                name,
                obj,
                &elf,
                &map,
                &mmp.dso_libraries(),
                std::slice::from_ref(&build_dir),
            );
            if objs.len() > 1 {
                let first = arg(obj);
                if let Some(pos) = link.iter().position(|a| a == &first) {
                    for extra in objs.iter().skip(1).rev() {
                        link.insert(pos + 1, arg(extra));
                    }
                }
            }
            self.run_tool(&link, &cwd)?;
            let out = build_dir.join(format!("{name}.{}", module.ext()));
            let frozen_def = Some(mmp.frozen_def(mmp_dir)?).filter(|p| module.dll && p.is_file());
            self.run_elf2e32(
                &self.elf2e32_args_for(
                    &module,
                    name,
                    &elf,
                    &out,
                    Some(&build_dir),
                    frozen_def.as_deref(),
                ),
                &cwd,
            )?;
            artifacts.push(if module.dll {
                Artifact::installed(out, format!("!:\\sys\\bin\\{name}.dll"))
            } else {
                Artifact::exe(out)
            });
            for res in &mmp.resource {
                artifacts.push(Artifact::installed(
                    build_dir.join(format!("{}.rsc", res.stem()?)),
                    res.install_dest()?,
                ));
            }
        }
        Ok(artifacts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fake() -> GcceBuild {
        GcceBuild {
            env: LocalEnv,
            tools: Toolchain {
                epocroot: PathBuf::from("/sdk"),
                gxx: PathBuf::from("/gcc/bin/arm-none-symbianelf-g++"),
                ld: PathBuf::from("/gcc/binutils/bin/arm-none-symbianelf-ld"),
                elf2e32: Some(PathBuf::from("/gcc/elf2e32")),
                gcc_lib: PathBuf::from("/gcc/lib/gcc/arm-none-symbianelf/12.1.0"),
                gcc_target_lib: PathBuf::from("/gcc/arm-none-symbianelf/lib"),
                wine: PathBuf::from("/usr/bin/wine"),
            },
            uid3: 0xe79e4cf9,
            capabilities: Vec::new(),
            icon: None,
        }
    }

    fn s(args: &[&str]) -> Vec<String> {
        args.iter().map(|a| (*a).to_string()).collect()
    }

    #[test]
    fn dll_module_uses_the_sdk_dll_recipe() {
        let d = fake();
        let mmp = Mmp::parse(
            "TARGET mathlib.dll\nTARGETTYPE DLL\nUID 0x1000008d 0xe5d1b001\nSOURCE m.cpp\n",
        )
        .unwrap();
        let module = Module::of(&mmp, 0xe79e_4cf9).unwrap();
        assert!(module.dll);
        let args = d.elf2e32_args_for(
            &module,
            "mathlib",
            Path::new("/p/build/mathlib.elf"),
            Path::new("/p/build/mathlib.dll"),
            Some(Path::new("/p/build")),
            None,
        );
        for want in [
            "--sid=0xe5d1b001",
            "--uid1=0x10000079",
            "--uid2=0x1000008d",
            "--uid3=0xe5d1b001",
            "--targettype=DLL",
            "--ignorenoncallable",
            "--dso=/p/build/mathlib.dso",
            "--defoutput=/p/build/mathlib.def",
            "--linkas=mathlib{000a0000}[e5d1b001].dll",
            "--libpath=/sdk/epoc32/release/armv5/lib;/p/build",
        ] {
            assert!(args.iter().any(|a| a == want), "missing {want}: {args:?}");
        }
        let job = symdev_elf2e32::Elf2E32::from_args(&args).unwrap();
        assert_eq!(job.uid().uid2, 0x1000_008d);
        let link = d.link_args_for(
            &module,
            "mathlib",
            Path::new("/p/build/m.o"),
            Path::new("/p/build/mathlib.elf"),
            Path::new("/p/build/mathlib.dll.map"),
            &[],
            &[PathBuf::from("/p/build")],
        );
        assert!(link.iter().any(|a| a == "-l:edll.lib"));
        assert!(link.iter().any(|a| a == "_E32Dll"));
        assert!(link.iter().any(|a| a == "mathlib{000a0000}[e5d1b001].dll"));
        let l = link.iter().position(|a| a == "-L/p/build").unwrap();
        let euser = link.iter().position(|a| a == "-l:euser.dso").unwrap();
        assert!(l < euser);
        let compile = d.compile_args_for(
            &module,
            Path::new("/p"),
            &CompileIncludes::default(),
            Path::new("/p/m.cpp"),
            Path::new("/p/build/m.o"),
        );
        assert!(compile.iter().any(|a| a == "-D__DLL__"));
        assert!(!compile.iter().any(|a| a == "-D__EXE__"));
    }

    #[test]
    fn dll_module_needs_uid_line() {
        let mmp = Mmp::parse("TARGET m.dll\nTARGETTYPE DLL\nSOURCE m.cpp\n").unwrap();
        assert!(Module::of(&mmp, 1).is_err());
    }

    #[test]
    fn link_args_append_mmp_libraries_once() {
        let d = fake();
        let args = d.link_args(
            "gui",
            Path::new("/p/build/gui.o"),
            Path::new("/p/build/gui.elf"),
            Path::new("/p/build/gui.exe.map"),
            &["euser.dso".into(), "avkon.dso".into()],
        );
        assert_eq!(args.iter().filter(|a| *a == "-l:euser.dso").count(), 1);
        let avkon = args.iter().position(|a| a == "-l:avkon.dso").unwrap();
        let supcpp = args.iter().position(|a| a == "-lsupc++").unwrap();
        assert!(avkon < supcpp);
    }

    #[test]
    fn compile_includes_wrap_the_sdk_include_dir() {
        let d = fake();
        let inc = CompileIncludes {
            user: vec![PathBuf::from("/p/build")],
            system: vec![PathBuf::from("/p/build/sdk-include-casefold")],
        };
        let args = d.compile_args(
            Path::new("/p/src"),
            &inc,
            Path::new("/p/src/gui.cpp"),
            Path::new("/p/build/gui.o"),
        );
        let at = |v: &str| args.iter().position(|a| a == v).unwrap();
        assert!(at("/p/src") < at("/p/build"));
        assert!(at("/p/build") < at("/sdk/epoc32/include"));
        assert!(at("/sdk/epoc32/include/variant") < at("/p/build/sdk-include-casefold"));
    }

    #[test]
    fn compile_args_match_recorded_experiment_5() {
        let d = fake();
        let args = d.compile_args(
            Path::new("/proj"),
            &CompileIncludes::default(),
            Path::new("/proj/hello.cpp"),
            Path::new("/proj/build/hello.o"),
        );
        assert_eq!(
            args,
            s(&[
                "/gcc/bin/arm-none-symbianelf-g++",
                "-O2",
                "-fexceptions",
                "-march=armv5t",
                "-mapcs",
                "-mthumb-interwork",
                "-mthumb",
                "-msoft-float",
                "-fpermissive",
                "-Wno-narrowing",
                "-D__SYMBIAN32__",
                "-D__EPOC32__",
                "-D__MARM__",
                "-D__GCCE__",
                "-D__EXE__",
                "-include",
                "/sdk/epoc32/include/gcce/gcce.h",
                "-D__PRODUCT_INCLUDE__=\"/sdk/epoc32/include/variant/symbian_os_v9.3.hrh\"",
                "-nostdinc",
                "-c",
                "-D__MARM_THUMB__",
                "-D__MARM_INTERWORK__",
                "-DNDEBUG",
                "-D_UNICODE",
                "-D__S60_3X__",
                "-D__SERIES60_3X__",
                "-D__EABI__",
                "-D__MARM_ARMV5__",
                "-D__SUPPORT_CPP_EXCEPTIONS__",
                "-I",
                "/proj",
                "-I",
                "/sdk/epoc32/include",
                "-I",
                "/sdk/epoc32/include/variant",
                "-I",
                "/gcc/lib/gcc/arm-none-symbianelf/12.1.0/include",
                "-o",
                "/proj/build/hello.o",
                "/proj/hello.cpp",
            ])
        );
        assert!(
            !args
                .iter()
                .any(|a| a.contains("-fPIC") || a.contains("-fPIE"))
        );
    }

    #[test]
    fn link_args_use_ld_2_29_1_recorded_experiment_5() {
        let d = fake();
        let args = d.link_args(
            "hello",
            Path::new("/proj/build/hello.o"),
            Path::new("/proj/build/hello.elf"),
            Path::new("/proj/build/hello.exe.map"),
            &[],
        );
        assert_eq!(
            args.first().map(String::as_str),
            Some("/gcc/binutils/bin/arm-none-symbianelf-ld")
        );
        assert_eq!(
            args,
            s(&[
                "/gcc/binutils/bin/arm-none-symbianelf-ld",
                "-L/gcc/lib/gcc/arm-none-symbianelf/12.1.0/",
                "-L",
                "/gcc/arm-none-symbianelf/lib",
                "--target1-abs",
                "--no-undefined",
                "-nostdlib",
                "-shared",
                "-Ttext",
                "0x8000",
                "-Tdata",
                "0x400000",
                "--default-symver",
                "-soname",
                "hello{000a0000}[e79e4cf9].exe",
                "--target1-abs",
                "--no-undefined",
                "-nostdlib",
                "--strip-debug",
                "--entry",
                "_E32Startup",
                "-u",
                "_E32Startup",
                "-L/sdk/epoc32/release/armv5/urel",
                "-l:eexe.lib",
                "-o",
                "/proj/build/hello.elf",
                "-Map",
                "/proj/build/hello.exe.map",
                "/proj/build/hello.o",
                "-(",
                "-l:usrt2_2.lib",
                "-)",
                "-L/gcc/arm-none-symbianelf/lib",
                "-L/sdk/epoc32/release/armv5/lib",
                "-l:euser.dso",
                "-l:dfpaeabi.dso",
                "-l:dfprvct2_2.dso",
                "-l:drtaeabi.dso",
                "-l:scppnwdl.dso",
                "-l:drtrvct2_2.dso",
                "-lsupc++",
                "-lgcc",
            ])
        );
        assert!(
            !args
                .iter()
                .any(|a| a.contains("-fPIC") || a.contains("-fPIE"))
        );
        assert!(
            !args
                .iter()
                .any(|a| a.contains("gcc-12.1.0/bin/arm-none-symbianelf-ld"))
        );
    }

    #[test]
    fn native_elf2e32_args_parse_as_the_recorded_job() {
        let mut d = fake();
        d.tools.elf2e32 = None;
        let args = d.elf2e32_args(
            "hello",
            Path::new("/p/build/hello.elf"),
            Path::new("/p/build/hello.exe"),
        );
        assert_eq!(args[0], "elf2e32");
        let job = symdev_elf2e32::Elf2E32::from_args(&args).unwrap();
        assert_eq!(job.uid3, 0xe79e4cf9);
        assert_eq!(job.output, PathBuf::from("/p/build/hello.exe"));
        assert_eq!(job.libpath, PathBuf::from("/sdk/epoc32/release/armv5/lib"));
        assert!(!job.uncompressed);
    }

    #[test]
    fn elf2e32_args_empty_capabilities_omit_flag() {
        let d = fake();
        let args = d.elf2e32_args(
            "hello",
            Path::new("/proj/build/hello.elf"),
            Path::new("/proj/build/hello.exe"),
        );
        assert_eq!(
            args,
            s(&[
                "/gcc/elf2e32",
                "--uid1=0x1000007a",
                "--uid3=0xe79e4cf9",
                "--fpu=softvfp",
                "--targettype=EXE",
                "--output=/proj/build/hello.exe",
                "--elfinput=/proj/build/hello.elf",
                "--linkas=hello{000a0000}[e79e4cf9].exe",
                "--libpath=/sdk/epoc32/release/armv5/lib",
            ])
        );
        assert!(!args.iter().any(|a| a.contains("--capability")));
    }

    #[test]
    fn resolve_source_prefers_mmp_dir_then_project_root() {
        let dir = tempfile::tempdir().unwrap();
        let mmp_dir = dir.path().join("group");
        std::fs::create_dir(&mmp_dir).unwrap();
        std::fs::write(mmp_dir.join("hello.cpp"), b"//").unwrap();
        let found = resolve_source(None, &mmp_dir, dir.path(), "hello.cpp").unwrap();
        assert_eq!(found, mmp_dir.join("hello.cpp"));
    }

    #[test]
    fn resolve_source_uses_sourcepath() {
        let dir = tempfile::tempdir().unwrap();
        let mmp_dir = dir.path().join("group");
        let src_dir = mmp_dir.join("src");
        std::fs::create_dir_all(&src_dir).unwrap();
        std::fs::write(src_dir.join("hello.cpp"), b"//").unwrap();
        let found = resolve_source(Some("src"), &mmp_dir, dir.path(), "hello.cpp").unwrap();
        assert_eq!(found, src_dir.join("hello.cpp"));
    }

    #[test]
    fn resolve_source_accepts_symbian_backslash_sourcepath() {
        let dir = tempfile::tempdir().unwrap();
        let mmp_dir = dir.path().join("group");
        std::fs::create_dir_all(dir.path().join("src")).unwrap();
        std::fs::create_dir_all(&mmp_dir).unwrap();
        std::fs::write(dir.path().join("src/app.cpp"), b"//").unwrap();
        let found = resolve_source(Some("..\\src"), &mmp_dir, dir.path(), "app.cpp").unwrap();
        assert_eq!(found, mmp_dir.join("../src/app.cpp"));
    }

    #[test]
    fn resolve_source_missing_errors() {
        let dir = tempfile::tempdir().unwrap();
        let err = resolve_source(None, dir.path(), dir.path(), "nope.cpp").unwrap_err();
        assert!(err.to_string().contains("source not found: nope.cpp"));
    }

    #[test]
    fn build_errors_without_bld_inf() {
        let dir = tempfile::tempdir().unwrap();
        let err = fake()
            .build(&Project {
                root: dir.path().to_path_buf(),
            })
            .unwrap_err();
        assert_eq!(err.to_string(), "no bld.inf");
    }

    #[test]
    fn build_errors_when_mmp_list_empty() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("bld.inf"), "PRJ_TESTMMPFILES\ntest.mmp\n").unwrap();
        let err = fake()
            .build(&Project {
                root: dir.path().to_path_buf(),
            })
            .unwrap_err();
        assert_eq!(err.to_string(), "no MMP to build");
    }

    #[test]
    fn elf2e32_args_join_manifest_capabilities_with_plus() {
        let mut d = fake();
        d.capabilities = vec!["ReadUserData".into(), "Location".into()];
        let args = d.elf2e32_args(
            "hello",
            Path::new("/proj/build/hello.elf"),
            Path::new("/proj/build/hello.exe"),
        );
        assert_eq!(
            args.iter()
                .find(|a| a.starts_with("--capability="))
                .map(String::as_str),
            Some("--capability=ReadUserData+Location")
        );
    }
}
