//! `symdev build`: one backend per manifest language.
use std::process::ExitCode;

use symdev_build::{FrozenExports, GcceBuild, RustBuild, RustSdk, Toolchain, UiResources};
use symdev_core::{BuildBackend, Error, LocalEnv};
use symdev_manifest::{Language, Manifest};

pub fn build_project(m: Manifest) -> Result<ExitCode, Error> {
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
    let tools = Toolchain::from_env()?;
    let epocroot = tools.epocroot.clone();
    let project = crate::current_project()?;
    // A `[ui]` project's icon is built by the Rust backend's own resource stage,
    // which names it after the application rather than after an MMP target there is
    // none of; `GcceBuild` must not also try, or `AppIcon::of` fails looking for one.
    let icon = m.symbian.icon.clone();
    let ui = m.ui.map(|ui| UiResources {
        app: m.package.name.clone(),
        uid3,
        ui,
        icon: icon.as_ref().map(|i| project.root.join(i)),
    });
    let gcce = GcceBuild {
        env: LocalEnv,
        tools,
        uid3,
        capabilities: m.symbian.capabilities,
        icon: if ui.is_some() { None } else { icon },
        icons: m.icons,
        secure_id: m.symbian.secure_id,
    };
    let artifacts = match m.language {
        Language::Cpp => gcce.build(&project)?,
        language => RustBuild {
            gcce,
            sdk: RustSdk::from_env()?,
            cargo: RustBuild::cargo_from_env(),
            rustc: RustBuild::rustc_from_env(),
            name: m.package.name,
            ui,
            std: language.has_std(),
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
