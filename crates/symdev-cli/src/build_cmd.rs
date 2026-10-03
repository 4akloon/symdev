//! `symdev build`: one backend per manifest language.
use std::path::Path;
use std::process::ExitCode;

use symdev_build::{FrozenExports, GcceBuild, StdSysroot};
use symdev_core::{BuildBackend, Error, LocalEnv};
use symdev_manifest::Manifest;

use crate::build_dir::BuildDir;
use crate::cargo_build::CargoBuild;
use crate::old_shape::OldShape;
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
    if m.language.is_rust() {
        return build_rust(&m, &project.root, provision);
    }
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
    for artifact in gcce.build(&project)? {
        println!("{}", artifact.path.display());
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

/// A Rust project: `cargo build --release` (design spec §3), after what cargo cannot do
/// itself — the nightly and SDK checks and `build/rust-sdk`, and for `rust-std` the
/// sysroot `symdev-rustc` points at. `symdev-ld` leaves `build/<name>.exe` and
/// `build/<name>.sisx`.
fn build_rust(m: &Manifest, root: &Path, provision: &Provision) -> Result<ExitCode, Error> {
    let read = |path: &str| std::fs::read_to_string(root.join(path)).unwrap_or_default();
    let (cargo, main, config) = (
        read("Cargo.toml"),
        read("src/main.rs"),
        read(".cargo/config.toml"),
    );
    if let Some(old) = OldShape::detect(
        &cargo,
        &main,
        &config,
        &m.package.name,
        m.language.has_std(),
    ) {
        return Err(Error::Other(old.message()));
    }
    let rust = RustProject::resolve(m, root, provision, true)?;
    BuildDir::of(root).create()?;
    rust.build.prepare(root)?;
    rust.build.check_link(root)?;
    if m.language.has_std() {
        StdSysroot::materialise(&rust.build.sdk, &rust.build.rustc, root)?;
    }
    CargoBuild::run(root)?;
    let build = root.join("build");
    for file in [
        format!("{}.exe", m.package.name),
        format!("{}.sisx", m.package.name),
    ] {
        let path = build.join(file);
        if !path.is_file() {
            // cargo found nothing to do, so symdev-ld did not run to write it again.
            return Err(Error::Other(format!(
                "{} is missing and cargo had nothing to relink: touch src/main.rs (or remove \
                 build/cargo) and build again",
                path.display()
            )));
        }
        println!("{}", path.display());
    }
    Ok(ExitCode::SUCCESS)
}
