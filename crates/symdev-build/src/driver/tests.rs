use std::path::PathBuf;

use super::*;

mod build;
mod compile;
mod container;
mod dll;
mod elf2e32_args;
mod gcce_compat;
mod language;
mod link;
mod resolve_source;
mod rust_build;

/// An SDK skeleton with just what preprocessing a project file needs: the variant
/// configuration, the header it names and an include directory.
fn fake_sdk(root: &std::path::Path) -> PathBuf {
    let epocroot = root.join("sdk");
    std::fs::create_dir_all(epocroot.join("epoc32/tools/variant")).unwrap();
    std::fs::create_dir_all(epocroot.join("epoc32/include/variant")).unwrap();
    std::fs::write(
        epocroot.join("epoc32/tools/variant/variant.cfg"),
        "epoc32\\include\\variant\\Symbian_OS_v9.3.hrh\n",
    )
    .unwrap();
    std::fs::write(
        epocroot.join("epoc32/include/variant/symbian_os_v9.3.hrh"),
        "#define __SECURE_SOFTWARE_INSTALL__\n",
    )
    .unwrap();
    epocroot
}

fn fake() -> GcceBuild {
    fake_at(PathBuf::from("/sdk"))
}

fn fake_at(epocroot: PathBuf) -> GcceBuild {
    GcceBuild {
        env: LocalEnv,
        tools: Toolchain {
            epocroot,
            gxx: PathBuf::from("/gcc/bin/arm-none-symbianelf-g++"),
            ld: PathBuf::from("/gcc/binutils/bin/arm-none-symbianelf-ld"),
            elf2e32: Some(PathBuf::from("/gcc/elf2e32")),
            gcc_lib: PathBuf::from("/gcc/lib/gcc/arm-none-symbianelf/12.1.0"),
            gcc_target_lib: PathBuf::from("/gcc/arm-none-symbianelf/lib"),
        },
        uid3: 0xe79e4cf9,
        capabilities: Vec::new(),
        icon: None,
        icons: Vec::new(),
    }
}

fn s(args: &[&str]) -> Vec<String> {
    args.iter().map(|a| (*a).to_string()).collect()
}
