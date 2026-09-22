mod artifacts;
mod build_cmd;
mod cli;
mod scaffold;
mod scaffold_rust;
mod test_cmd;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{CommandFactory, Parser};
use symdev_build::{AppTarget, Epocroot, FrozenExports, SisPackage, UiResources};
use symdev_core::{Error, PackageBackend, Project};

use artifacts::package_artifacts;
use cli::{Cli, Commands};

fn main() -> ExitCode {
    match Cli::parse().command {
        None => {
            let _ = Cli::command().print_help();
            ExitCode::SUCCESS
        }
        Some(Commands::New {
            name,
            template,
            lang,
            ..
        }) => {
            match std::env::current_dir()
                .map_err(|e| Error::Other(e.to_string()))
                .and_then(|cwd| scaffold::create_project(&cwd, &name, template, lang))
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
            Ok(m) => match build_cmd::build_project(m) {
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
        Some(Commands::Test { emulator }) => {
            match symdev_manifest::load(Path::new("symdev.toml")) {
                Ok(m) => match test_cmd::test_project(m, emulator) {
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
            }
        }
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

/// The SDK root, which only a project with a `bld.inf` needs: reading one runs the
/// preprocessor, and that wants the SDK include directory and the variant header.
fn epocroot_for(project: &Project) -> Result<PathBuf, Error> {
    if AppTarget::has_bld_inf(project) {
        return Ok(Epocroot::from_env()?.path().to_path_buf());
    }
    Ok(PathBuf::new())
}

pub(crate) fn current_project() -> Result<Project, Error> {
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
    // The same caption translations the build compiled, so the package installs them.
    let locales = symdev_locale::Locales::load(&cwd.join("locales"))
        .map_err(|e| Error::Other(e.to_string()))?;
    let ui = m.ui.clone().map(|ui| {
        UiResources {
            app: app.name().to_string(),
            uid3,
            ui,
            icon: icon.as_ref().map(|i| cwd.join(i)),
            captions: Vec::new(),
        }
        .with_locales(locales.as_ref())
    });
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
        if ui.is_some() { None } else { icon.as_deref() },
        &m.icons,
        &m.install,
        &epocroot,
        ui.as_ref(),
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
