use std::path::PathBuf;

use super::*;

mod build;
mod compile;
mod dll;
mod elf2e32_args;
mod gcce_compat;
mod language;
mod link;
mod resolve_source;

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
        },
        uid3: 0xe79e4cf9,
        capabilities: Vec::new(),
        icon: None,
    }
}

fn s(args: &[&str]) -> Vec<String> {
    args.iter().map(|a| (*a).to_string()).collect()
}
