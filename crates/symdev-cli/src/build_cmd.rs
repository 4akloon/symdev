//! `symdev build`: one backend per manifest language.
use std::process::ExitCode;

use symdev_build::{FrozenExports, GcceBuild};
use symdev_core::{BuildBackend, Error, LocalEnv};
use symdev_manifest::{Language, Manifest};

use crate::build_dir::BuildDir;
use crate::provision::Provision;
use crate::rust_project::RustProject;

pub fn build_project(m: Manifest, provision: &Provision) -> Result<ExitCode, Error> {
    let uid3 = m
        .symbian
        .uid3
        .ok_or_else(|| Error::Other("uid3 required for build (set symbian.uid3)".into()))?;
    if m.ui.is_some() && !m.language.is_rust() {
        return Err(Error::Other(
            "[ui] is for a `language = \"rust\"` project: a C++ project declares its \
             application resources in its .mmp with START RESOURCE, and symdev would \
             generate a second, conflicting pair from this section"
                .into(),
        ));
    }
    let project = crate::current_project()?;
    let (artifacts, epocroot) = if m.language.is_rust() {
        let rust = RustProject::resolve(&m, &project.root, provision, true)?;
        BuildDir::of(&project.root).create()?;
        (rust.build.build(&project)?, rust.epocroot)
    } else {
        let tools = provision.toolchain(m.target.device, true)?;
        let epocroot = tools.epocroot.clone();
        BuildDir::of(&project.root).create()?;
        let gcce = GcceBuild {
            env: LocalEnv,
            tools,
            uid3,
            capabilities: m.symbian.capabilities,
            icon: m.symbian.icon,
            icons: m.icons,
            secure_id: m.symbian.secure_id,
        };
        (gcce.build(&project)?, epocroot)
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
