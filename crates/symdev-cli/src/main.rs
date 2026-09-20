mod cli;
mod scaffold;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{CommandFactory, Parser};
use symdev_build::{
    AppIcon, AppTarget, BuildOutputs, Epocroot, FrozenExports, GcceBuild, SisPackage, Toolchain,
};
use symdev_core::{Artifact, BuildBackend, Error, LocalEnv, PackageBackend, Project};

use cli::{Cli, Commands};

fn main() -> ExitCode {
    match Cli::parse().command {
        None => {
            let _ = Cli::command().print_help();
            ExitCode::SUCCESS
        }
        Some(Commands::New { name, template, .. }) => {
            match std::env::current_dir()
                .map_err(|e| Error::Other(e.to_string()))
                .and_then(|cwd| scaffold::create_project(&cwd, &name, template))
            {
                Ok(root) => {
                    println!("{}", root.display());
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("error: {e}");
                    ExitCode::from(1)
                }
            }
        }
        Some(Commands::Build) => match symdev_manifest::load(Path::new("symdev.toml")) {
            Ok(m) => match build_project(m) {
                Ok(code) => code,
                Err(e) => {
                    eprintln!("error: {e}");
                    ExitCode::from(1)
                }
            },
            Err(e) => {
                eprintln!("error: invalid manifest: {e}");
                ExitCode::from(1)
            }
        },
        Some(Commands::Package) => match symdev_manifest::load(Path::new("symdev.toml")) {
            Ok(m) => match package_project(m) {
                Ok(code) => code,
                Err(e) => {
                    eprintln!("error: {e}");
                    ExitCode::from(1)
                }
            },
            Err(e) => {
                eprintln!("error: invalid manifest: {e}");
                ExitCode::from(1)
            }
        },
        Some(Commands::Run) => match symdev_manifest::load(Path::new("symdev.toml")) {
            Ok(m) => match run_project(m) {
                Ok(code) => code,
                Err(e) => {
                    eprintln!("error: {e}");
                    ExitCode::from(1)
                }
            },
            Err(e) => {
                eprintln!("error: invalid manifest: {e}");
                ExitCode::from(1)
            }
        },
        Some(Commands::Freeze) => match freeze_project() {
            Ok(code) => code,
            Err(e) => {
                eprintln!("error: {e}");
                ExitCode::from(1)
            }
        },
        Some(Commands::Deploy) => match symdev_manifest::load(Path::new("symdev.toml")) {
            Ok(m) => match deploy_project(m) {
                Ok(code) => code,
                Err(e) => {
                    eprintln!("error: {e}");
                    ExitCode::from(1)
                }
            },
            Err(e) => {
                eprintln!("error: invalid manifest: {e}");
                ExitCode::from(1)
            }
        },
    }
}

fn build_project(m: symdev_manifest::Manifest) -> Result<ExitCode, Error> {
    let uid3 = m
        .symbian
        .uid3
        .ok_or_else(|| Error::Other("uid3 required for build (set symbian.uid3)".into()))?;
    let tools = Toolchain::from_env()?;
    let epocroot = tools.epocroot.clone();
    let artifacts = pollster::block_on(async {
        GcceBuild {
            env: LocalEnv,
            tools,
            uid3,
            capabilities: m.symbian.capabilities,
            icon: m.symbian.icon,
        }
        .build(&Project {
            root: std::env::current_dir().map_err(|e| Error::Other(e.to_string()))?,
        })
    })?;
    for artifact in artifacts {
        println!("{}", artifact.path.display());
    }
    for dll in FrozenExports::of(&current_project()?, &epocroot)? {
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

/// The SDK root, which only a project with a `bld.inf` needs: reading one runs the
/// preprocessor, and that wants the SDK include directory and the variant header.
fn epocroot_for(project: &Project) -> Result<PathBuf, Error> {
    if AppTarget::has_bld_inf(project) {
        return Ok(Epocroot::from_env()?.path().to_path_buf());
    }
    Ok(PathBuf::new())
}

fn current_project() -> Result<Project, Error> {
    Ok(Project {
        root: std::env::current_dir().map_err(|e| Error::Other(e.to_string()))?,
    })
}

fn freeze_project() -> Result<ExitCode, Error> {
    let project = current_project()?;
    let changed = FrozenExports::freeze(&project, &epocroot_for(&project)?)?;
    if changed.is_empty() {
        println!("exports already frozen");
    }
    for dll in changed {
        println!(
            "{}: froze {} in {}",
            dll.dll,
            dll.unfrozen.join(", "),
            dll.frozen_def.display()
        );
    }
    Ok(ExitCode::SUCCESS)
}

fn package_project(m: symdev_manifest::Manifest) -> Result<ExitCode, Error> {
    let uid3 = m
        .symbian
        .uid3
        .ok_or_else(|| Error::Other("uid3 required for package (set symbian.uid3)".into()))?;
    let cwd = std::env::current_dir().map_err(|e| Error::Other(e.to_string()))?;
    let project = Project { root: cwd.clone() };
    let epocroot = epocroot_for(&project)?;
    let app = AppTarget::of(&project, &m.package.name, &epocroot)?;
    let e32 = PathBuf::from("build").join(app.exe_file());
    if !e32.is_file() {
        return Err(Error::Other(format!(
            "E32 not found: {} (run symdev build)",
            e32.display()
        )));
    }
    let icon = m.symbian.icon.clone();
    let password = std::env::var("SYMDEV_SIGN_PASSWORD").unwrap_or_default();
    let package = SisPackage {
        name: m.package.name,
        app: app.name().to_string(),
        uid3,
        version: m.package.version,
        vendor: m.symbian.vendor,
        capabilities: m.symbian.capabilities,
        password,
        cert: m
            .signing
            .cert
            .map(|p| if p.is_absolute() { p } else { cwd.join(p) }),
        key: m
            .signing
            .key
            .map(|p| if p.is_absolute() { p } else { cwd.join(p) }),
        subject: m.signing.subject,
    }
    .package(&package_artifacts(
        &project,
        &e32,
        icon.as_deref(),
        &m.install,
        &epocroot,
    )?)?;
    println!("{}", package.primary.display());
    Ok(ExitCode::SUCCESS)
}

fn deploy_project(m: symdev_manifest::Manifest) -> Result<ExitCode, Error> {
    let sisx = PathBuf::from("build").join(format!("{}.sisx", m.package.name));
    if !sisx.is_file() {
        return Err(Error::Other(format!(
            "SISX not found: build/{}.sisx (run symdev package)",
            m.package.name
        )));
    }
    let cwd = std::env::current_dir().map_err(|e| Error::Other(e.to_string()))?;
    println!("{}", cwd.join(&sisx).display());
    Ok(ExitCode::SUCCESS)
}

fn run_project(m: symdev_manifest::Manifest) -> Result<ExitCode, Error> {
    let uid3 = m
        .symbian
        .uid3
        .ok_or_else(|| Error::Other("uid3 required for run (set symbian.uid3)".into()))?;
    let cwd = std::env::current_dir().map_err(|e| Error::Other(e.to_string()))?;
    let sisx = cwd.join("build").join(format!("{}.sisx", m.package.name));
    if !sisx.is_file() {
        return Err(Error::Other(format!(
            "SISX not found: build/{}.sisx (run symdev package)",
            m.package.name
        )));
    }
    let emulator = symdev_emulator::Eka2l1Backend::from_env()?;
    let log = cwd.join("build").join("eka2l1.log");
    let pid_file = cwd.join("build").join("eka2l1.pid");
    if let Some(old) = symdev_emulator::Eka2l1Backend::previous(&pid_file) {
        eprintln!(
            "warning: EKA2L1 from the previous run (pid {old}) is still open; close its window \
             (it ignores SIGTERM) to avoid two emulators on the same data"
        );
    }
    let pid = emulator.run(&sisx, uid3, &log)?;
    std::fs::write(&pid_file, pid.to_string()).map_err(|e| Error::Other(e.to_string()))?;
    println!(
        "EKA2L1 pid {pid}: installing {} and launching 0x{uid3:08x}",
        sisx.display()
    );
    println!("log: {}", log.display());
    Ok(ExitCode::SUCCESS)
}

/// The EXE, the resources the project's MMPs compile (`BuildOutputs`), the icon and the
/// manifest's `[[install]]` files; a project without `bld.inf` packages its EXE and its
/// `[[install]]` files alone.
fn package_artifacts(
    project: &Project,
    e32: &Path,
    icon: Option<&Path>,
    install: &[symdev_manifest::InstallFile],
    epocroot: &Path,
) -> Result<Vec<Artifact>, Error> {
    let cwd = &project.root;
    let mut outputs = if AppTarget::has_bld_inf(project) {
        let mut outputs = BuildOutputs::of(project, epocroot)?;
        if let Some(source) = icon {
            outputs.push(AppIcon::of(project, source, epocroot)?.artifact(&cwd.join("build")));
        }
        outputs
            .into_iter()
            .filter(|a| a.dest.is_some() || a.path == cwd.join(e32))
            .collect()
    } else {
        vec![Artifact::exe(cwd.join(e32))]
    };
    for file in install {
        outputs.push(Artifact::installed(
            cwd.join(&file.source),
            file.dest.clone(),
        ));
    }
    for a in &outputs {
        if a.dest.is_some() && !a.path.is_file() {
            return Err(Error::Other(format!(
                "file to install not found: {} (run symdev build)",
                a.path.display()
            )));
        }
    }
    Ok(outputs)
}
