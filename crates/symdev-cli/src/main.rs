mod cli;
mod scaffold;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{CommandFactory, Parser};
use symdev_build::{GcceBuild, SisPackage, Toolchain};
use symdev_core::{Artifact, BuildBackend, Error, LocalEnv, PackageBackend, Project};

use cli::{Cli, Commands};

fn main() -> ExitCode {
    match Cli::parse().command {
        None => {
            let _ = Cli::command().print_help();
            ExitCode::SUCCESS
        }
        Some(Commands::New { name, .. }) => {
            match std::env::current_dir()
                .map_err(|e| Error::Other(e.to_string()))
                .and_then(|cwd| scaffold::create_project(&cwd, &name))
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
    let artifacts = pollster::block_on(async {
        GcceBuild {
            env: LocalEnv,
            tools,
            uid3,
            capabilities: m.symbian.capabilities,
        }
        .build(&Project {
            root: std::env::current_dir().map_err(|e| Error::Other(e.to_string()))?,
        })
    })?;
    for artifact in artifacts {
        println!("{}", artifact.path.display());
    }
    Ok(ExitCode::SUCCESS)
}

fn package_project(m: symdev_manifest::Manifest) -> Result<ExitCode, Error> {
    let uid3 = m
        .symbian
        .uid3
        .ok_or_else(|| Error::Other("uid3 required for package (set symbian.uid3)".into()))?;
    let e32 = PathBuf::from("build").join(format!("{}.exe", m.package.name));
    if !e32.is_file() {
        return Err(Error::Other(format!(
            "E32 not found: build/{}.exe (run symdev build)",
            m.package.name
        )));
    }
    let password = std::env::var("SYMDEV_SIGN_PASSWORD").unwrap_or_default();
    let cwd = std::env::current_dir().map_err(|e| Error::Other(e.to_string()))?;
    let package = SisPackage {
        name: m.package.name,
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
    .package(&[Artifact {
        path: cwd.join(&e32),
    }])?;
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
    let pid = emulator.run(&sisx, uid3, &log)?;
    println!(
        "EKA2L1 pid {pid}: installing {} and launching 0x{uid3:08x}",
        sisx.display()
    );
    println!("log: {}", log.display());
    Ok(ExitCode::SUCCESS)
}
