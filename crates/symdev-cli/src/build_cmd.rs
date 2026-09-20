//! `symdev build`: one backend per manifest language.
use std::process::ExitCode;

use symdev_build::{FrozenExports, GcceBuild, RustBuild, RustSdk, Toolchain};
use symdev_core::{BuildBackend, Error, LocalEnv};
use symdev_manifest::{Language, Manifest};

pub fn build_project(m: Manifest) -> Result<ExitCode, Error> {
    let uid3 = m
        .symbian
        .uid3
        .ok_or_else(|| Error::Other("uid3 required for build (set symbian.uid3)".into()))?;
    let tools = Toolchain::from_env()?;
    let epocroot = tools.epocroot.clone();
    let gcce = GcceBuild {
        env: LocalEnv,
        tools,
        uid3,
        capabilities: m.symbian.capabilities,
        icon: m.symbian.icon,
        icons: m.icons,
        secure_id: m.symbian.secure_id,
    };
    let project = crate::current_project()?;
    let artifacts = match m.language {
        Language::Cpp => gcce.build(&project)?,
        Language::Rust => RustBuild {
            gcce,
            sdk: RustSdk::from_env()?,
            cargo: RustBuild::cargo_from_env(),
            name: m.package.name,
        }
        .build(&project)?,
    };
    for artifact in artifacts {
        println!("{}", artifact.path.display());
    }
    if m.language != Language::Cpp {
        return Ok(ExitCode::SUCCESS);
    }
    for dll in FrozenExports::of(&project, &epocroot)? {
        if !dll.unfrozen.is_empty() {
            eprintln!(
                "warning: {}: {} export(s) not frozen in {} ({}); run `symdev freeze` \
                 before shipping so their ordinals stay fixed",
                dll.dll,
                dll.unfrozen.len(),
                dll.frozen_def.display(),
                dll.unfrozen.join(", ")
            );
        }
    }
    Ok(ExitCode::SUCCESS)
}
