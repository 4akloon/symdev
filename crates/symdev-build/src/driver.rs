use std::path::{Path, PathBuf};
use std::process::Command;

use symdev_core::{Artifact, BuildBackend, Error, LocalEnv, Project, RemotePath, Result};

use crate::toolchain::Toolchain;
use crate::{parse_bld_inf, parse_mmp};

pub struct GcceBuild {
    pub env: LocalEnv,
    pub tools: Toolchain,
    pub uid3: u32,
    pub capabilities: Vec<String>,
}

fn arg(path: &Path) -> String {
    path.display().to_string()
}

impl GcceBuild {
    fn linkas(&self, name: &str) -> String {
        format!("{name}{{000a0000}}[{:08x}].exe", self.uid3)
    }

    pub fn compile_args(&self, source_dir: &Path, source: &Path, obj: &Path) -> Vec<String> {
        let epoc = &self.tools.epocroot;
        vec![
            arg(&self.tools.gxx),
            "-O2".into(),
            "-fexceptions".into(),
            "-march=armv5t".into(),
            "-mapcs".into(),
            "-mthumb-interwork".into(),
            "-mthumb".into(),
            "-msoft-float".into(),
            "-D__SYMBIAN32__".into(),
            "-D__EPOC32__".into(),
            "-D__MARM__".into(),
            "-D__GCCE__".into(),
            "-D__EXE__".into(),
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
            "-I".into(),
            arg(&epoc.join("epoc32/include")),
            "-I".into(),
            arg(&epoc.join("epoc32/include/variant")),
            "-I".into(),
            arg(&self.tools.gcc_lib.join("include")),
            "-o".into(),
            arg(obj),
            arg(source),
        ]
    }

    pub fn link_args(&self, name: &str, obj: &Path, elf: &Path, map: &Path) -> Vec<String> {
        vec![
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
            self.linkas(name),
            "--target1-abs".into(),
            "--no-undefined".into(),
            "-nostdlib".into(),
            "--strip-debug".into(),
            "--entry".into(),
            "_E32Startup".into(),
            "-u".into(),
            "_E32Startup".into(),
            format!(
                "-L{}",
                self.tools
                    .epocroot
                    .join("epoc32/release/armv5/urel")
                    .display()
            ),
            "-l:eexe.lib".into(),
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
            "-lsupc++".into(),
            "-lgcc".into(),
        ]
    }

    pub fn elf2e32_args(&self, name: &str, elf: &Path, exe: &Path) -> Vec<String> {
        let mut args = vec![
            arg(&self.tools.elf2e32),
            "--uid1=0x1000007a".into(),
            format!("--uid3=0x{:08x}", self.uid3),
        ];
        // Empty `--capability=` is rejected: "Option capability has missed argument"
        // (elf2e32_next 3.0 Build 2). Omit the flag when the manifest list is empty.
        if !self.capabilities.is_empty() {
            args.push(format!("--capability={}", self.capabilities.join("+")));
        }
        args.extend([
            "--fpu=softvfp".into(),
            "--targettype=EXE".into(),
            format!("--output={}", exe.display()),
            format!("--elfinput={}", elf.display()),
            format!("--linkas={}", self.linkas(name)),
            format!(
                "--libpath={}",
                self.tools
                    .epocroot
                    .join("epoc32/release/armv5/lib")
                    .display()
            ),
        ]);
        args
    }

    fn run_tool(&self, args: &[String], cwd: &RemotePath) -> Result<()> {
        let mut cmd = Command::new(&args[0]);
        cmd.args(&args[1..]);
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
    sourcepath: &[String],
    mmp_dir: &Path,
    project_root: &Path,
    source: &str,
) -> Result<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(sp) = sourcepath.last() {
        let sp = Path::new(sp);
        if sp.is_absolute() {
            candidates.push(sp.join(source));
        } else {
            candidates.push(mmp_dir.join(sp).join(source));
        }
    }
    candidates.push(mmp_dir.join(source));
    candidates.push(project_root.join(source));
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
        let group = project.root.join("group").join("bld.inf");
        let root = project.root.join("bld.inf");
        let bld_path = if group.is_file() {
            group
        } else if root.is_file() {
            root
        } else {
            return Err(Error::Other("no bld.inf".into()));
        };
        let bld = parse_bld_inf(&std::fs::read_to_string(&bld_path).map_err(io)?)
            .map_err(|e| Error::Other(e.to_string()))?;
        if bld.mmp_files.is_empty() {
            return Err(Error::Other("no MMP to build".into()));
        }
        let bld_dir = bld_path.parent().unwrap_or(&project.root);
        let build_dir = project.root.join("build");
        std::fs::create_dir_all(&build_dir).map_err(io)?;
        let cwd = RemotePath::new(arg(&project.root));

        let mut artifacts = Vec::new();
        for mmp_rel in &bld.mmp_files {
            let mmp_path = bld_dir.join(mmp_rel);
            let mmp = parse_mmp(&std::fs::read_to_string(&mmp_path).map_err(io)?)
                .map_err(|e| Error::Other(e.to_string()))?;
            let mmp_dir = mmp_path.parent().unwrap_or(bld_dir);
            let name = Path::new(&mmp.target)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or(mmp.target.as_str());

            let mut objs = Vec::new();
            for src in &mmp.source {
                let source = resolve_source(&mmp.sourcepath, mmp_dir, &project.root, src)?;
                let source_dir = source.parent().unwrap_or(mmp_dir);
                let stem = source
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or(src.as_str());
                let obj = build_dir.join(format!("{stem}.o"));
                self.run_tool(&self.compile_args(source_dir, &source, &obj), &cwd)?;
                objs.push(obj);
            }
            let obj = objs
                .first()
                .ok_or_else(|| Error::Other("no SOURCE".into()))?;
            let elf = build_dir.join(format!("{name}.elf"));
            let map = build_dir.join(format!("{name}.exe.map"));
            let mut link = self.link_args(name, obj, &elf, &map);
            if objs.len() > 1 {
                let first = arg(obj);
                if let Some(pos) = link.iter().position(|a| a == &first) {
                    for extra in objs.iter().skip(1).rev() {
                        link.insert(pos + 1, arg(extra));
                    }
                }
            }
            self.run_tool(&link, &cwd)?;
            let exe = build_dir.join(format!("{name}.exe"));
            self.run_tool(&self.elf2e32_args(name, &elf, &exe), &cwd)?;
            artifacts.push(Artifact { path: exe });
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
                elf2e32: PathBuf::from("/gcc/elf2e32"),
                gcc_lib: PathBuf::from("/gcc/lib/gcc/arm-none-symbianelf/12.1.0"),
                gcc_target_lib: PathBuf::from("/gcc/arm-none-symbianelf/lib"),
            },
            uid3: 0xe79e4cf9,
            capabilities: Vec::new(),
        }
    }

    fn s(args: &[&str]) -> Vec<String> {
        args.iter().map(|a| (*a).to_string()).collect()
    }

    #[test]
    fn compile_args_match_recorded_experiment_5() {
        let d = fake();
        let args = d.compile_args(
            Path::new("/proj"),
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
        let found = resolve_source(&[], &mmp_dir, dir.path(), "hello.cpp").unwrap();
        assert_eq!(found, mmp_dir.join("hello.cpp"));
    }

    #[test]
    fn resolve_source_uses_sourcepath() {
        let dir = tempfile::tempdir().unwrap();
        let mmp_dir = dir.path().join("group");
        let src_dir = mmp_dir.join("src");
        std::fs::create_dir_all(&src_dir).unwrap();
        std::fs::write(src_dir.join("hello.cpp"), b"//").unwrap();
        let found = resolve_source(
            &["src".into()],
            &mmp_dir,
            dir.path(),
            "hello.cpp",
        )
        .unwrap();
        assert_eq!(found, src_dir.join("hello.cpp"));
    }

    #[test]
    fn resolve_source_missing_errors() {
        let dir = tempfile::tempdir().unwrap();
        let err = resolve_source(&[], dir.path(), dir.path(), "nope.cpp").unwrap_err();
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
