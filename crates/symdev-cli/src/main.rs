mod cli;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{CommandFactory, Parser};
use symdev_build::{GcceBuild, SisPackage, SisTools, Toolchain};
use symdev_core::{Artifact, BuildBackend, Error, LocalEnv, PackageBackend, Project};

use cli::{Cli, Commands};

fn main() -> ExitCode {
    match Cli::parse().command {
        None => {
            let _ = Cli::command().print_help();
            ExitCode::SUCCESS
        }
        Some(Commands::New { .. }) => not_implemented("symdev new", "M4"),
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
        Some(Commands::Deploy) => require_manifest("symdev deploy", "M4"),
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
    let tools = SisTools::from_env()?;
    let password = std::env::var("SYMDEV_SIGN_PASSWORD").unwrap_or_default();
    let cwd = std::env::current_dir().map_err(|e| Error::Other(e.to_string()))?;
    let package = SisPackage {
        env: LocalEnv,
        tools,
        name: m.package.name,
        uid3,
        version: m.package.version,
        vendor: m.symbian.vendor,
        password,
        cert: m
            .signing
            .cert
            .map(|p| if p.is_absolute() { p } else { cwd.join(p) }),
        key: m
            .signing
            .key
            .map(|p| if p.is_absolute() { p } else { cwd.join(p) }),
    }
    .package(&[Artifact {
        path: cwd.join(&e32),
    }])?;
    println!("{}", package.primary.display());
    Ok(ExitCode::SUCCESS)
}

fn require_manifest(feature: &'static str, milestone: &'static str) -> ExitCode {
    match symdev_manifest::load(Path::new("symdev.toml")) {
        Ok(_) => not_implemented(feature, milestone),
        Err(e) => {
            eprintln!("error: invalid manifest: {e}");
            ExitCode::from(1)
        }
    }
}

fn not_implemented(feature: &'static str, milestone: &'static str) -> ExitCode {
    let err = Error::NotImplemented { feature, milestone };
    eprintln!("error: {err}");
    ExitCode::from(1)
}
