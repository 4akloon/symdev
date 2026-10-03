mod artifacts;
mod build_cmd;
mod build_dir;
mod cli;
mod devices_cmd;
mod ld;
mod provision;
mod role;
mod run;
mod rust_project;
mod rustc_wrapper;
mod scaffold;
mod scaffold_rust;
mod sdk_cmd;
mod setup_linker;
mod sisx;
mod test_cmd;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{CommandFactory, Parser};
use symdev_build::{AppTarget, Epocroot, FrozenExports};
use symdev_core::{Error, Project};

use cli::{Cli, Commands, EmulatorAction};
use provision::Provision;
use role::Role;
use sisx::ProjectPackage;

fn main() -> ExitCode {
    let mut args = std::env::args_os();
    let argv0 = args.next().unwrap_or_default();
    match Role::of(&argv0) {
        Role::Linker => {
            return exit(
                ld::LinkRun::from_env(args)
                    .and_then(|r| r.run())
                    .map(|()| ExitCode::SUCCESS),
            );
        }
        Role::Rustc => {
            return exit(
                rustc_wrapper::RustcWrapper::run(Path::new(&argv0), args).map(|n| match n {}),
            );
        }
        Role::Cli => {}
    }
    let cli = Cli::parse();
    let provision = Provision::from_env(cli.offline);
    let result = match cli.command {
        None => {
            let _ = Cli::command().print_help();
            return ExitCode::SUCCESS;
        }
        Some(Commands::New {
            name,
            template,
            lang,
            ..
        }) => std::env::current_dir()
            .map_err(|e| Error::Other(e.to_string()))
            .and_then(|cwd| {
                scaffold::create_project(&cwd, &name, template, lang, || provision.rust_sdk())
            })
            .map(|root| {
                println!("{}", root.display());
                ExitCode::SUCCESS
            }),
        Some(Commands::Build) => manifest().and_then(|m| build_cmd::build_project(m, &provision)),
        Some(Commands::Package) => manifest().and_then(|m| package_project(m, &provision)),
        Some(Commands::Run { exe, args }) => run::run(exe, args),
        Some(Commands::Test { emulator }) => {
            manifest().and_then(|m| test_cmd::test_project(m, emulator))
        }
        Some(Commands::Freeze) => freeze_project(&provision),
        Some(Commands::Deploy) => manifest().and_then(deploy_project),
        Some(Commands::Sdk { action }) => sdk_cmd::run(action, &provision),
        Some(Commands::SetupLinker { dir }) => setup_linker::setup_linker(dir),
        Some(Commands::Devices) => devices_cmd::list(),
        Some(Commands::Emulator { action }) => match action {
            EmulatorAction::Start { profile } => devices_cmd::start(&profile),
            EmulatorAction::Stop { id } => devices_cmd::stop(&id),
        },
    };
    exit(result)
}

/// `error: <e>` and status 1, or the command's own status.
fn exit(result: Result<ExitCode, Error>) -> ExitCode {
    result.unwrap_or_else(|e| {
        eprintln!("error: {e}");
        ExitCode::from(1)
    })
}

/// The project's `symdev.toml`.
pub(crate) fn manifest() -> Result<symdev_manifest::Manifest, Error> {
    symdev_manifest::load(Path::new("symdev.toml"))
        .map_err(|e| Error::Other(format!("invalid manifest: {e}")))
}

/// The SDK root, which only a project with a `bld.inf` needs: reading one runs the
/// preprocessor, and that wants the SDK include directory and the variant header.
fn epocroot_for(
    project: &Project,
    sdk: impl FnOnce() -> Result<Epocroot, Error>,
) -> Result<PathBuf, Error> {
    if AppTarget::has_bld_inf(project) {
        return Ok(sdk()?.path().to_path_buf());
    }
    Ok(PathBuf::new())
}

pub(crate) fn current_project() -> Result<Project, Error> {
    Ok(Project {
        root: std::env::current_dir().map_err(|e| Error::Other(e.to_string()))?,
    })
}

fn freeze_project(provision: &Provision) -> Result<ExitCode, Error> {
    let project = current_project()?;
    let device = || Ok(manifest()?.target.device);
    let epocroot = epocroot_for(&project, || provision.epocroot(device))?;
    let changed = FrozenExports::freeze(&project, &epocroot)?;
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

fn package_project(m: symdev_manifest::Manifest, provision: &Provision) -> Result<ExitCode, Error> {
    let cwd = std::env::current_dir().map_err(|e| Error::Other(e.to_string()))?;
    let project = Project { root: cwd.clone() };
    let device = m.target.device;
    let epocroot = epocroot_for(&project, || provision.installed_epocroot(device))?;
    let package = ProjectPackage::new(m, cwd.clone(), epocroot)?;
    let e32 = cwd.join("build").join(format!("{}.exe", package.app()));
    if !e32.is_file() {
        return Err(Error::Other(format!(
            "E32 not found: build/{}.exe (run symdev build)",
            package.app()
        )));
    }
    let password = std::env::var("SYMDEV_SIGN_PASSWORD").unwrap_or_default();
    println!("{}", package.package(&e32, &password)?.display());
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
