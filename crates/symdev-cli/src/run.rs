//! The runner: `symdev run --exe <image>` is cargo's runner (`.cargo/config.toml`), and
//! `symdev run` runs the project's own package (design spec §6).
mod app_exit;
mod device_pick;
mod exe_target;
mod interrupt;
mod log_tail;
mod runner;

use std::io::IsTerminal;
use std::path::PathBuf;
use std::process::ExitCode;

use symdev_core::{Error, Result};
use symdev_emulator::control::ControlClient;

pub(crate) use app_exit::AppExit;
pub(crate) use device_pick::pick_device;
pub(crate) use exe_target::ExeTarget;
pub(crate) use interrupt::Interrupt;
pub(crate) use log_tail::LogTail;
pub(crate) use runner::Runner;

pub(crate) fn run(exe: Option<PathBuf>, args: Vec<String>) -> Result<ExitCode> {
    refuse_arguments(&args)?;
    let cwd = std::env::current_dir().map_err(|e| Error::Other(e.to_string()))?;
    let target = match exe {
        Some(exe) => ExeTarget::of(&exe, &cwd)?,
        None => project_target(&cwd)?,
    };
    let device = pick_device(std::io::stdin().is_terminal())?;
    let client = ControlClient::connect(&device.socket)?;
    // Only now: until the app runs, Ctrl+C ends symdev as usual.
    let interrupt = Interrupt::install()?;
    let exit = Runner::new(target, device, client).run(&interrupt, &mut std::io::stdout())?;
    if let Some(message) = &exit.message {
        eprintln!("{message}");
    }
    Ok(ExitCode::from(exit.code))
}

/// Spec §6.6: passing a command line to an app was never observed on the real system.
pub(crate) fn refuse_arguments(args: &[String]) -> Result<()> {
    match args {
        [] => Ok(()),
        _ => Err(Error::Other(format!(
            "TODO: arguments for the app ({}) (not observed): passing a command line to a \
             Symbian app was never observed on the real system",
            args.join(" ")
        ))),
    }
}

/// `build/<name>.sisx` of the project in `cwd`, for `symdev run` without `--exe`.
fn project_target(cwd: &std::path::Path) -> Result<ExeTarget> {
    let m = crate::manifest()?;
    let uid3 = m
        .symbian
        .uid3
        .ok_or_else(|| Error::Other("uid3 required for run (set symbian.uid3)".into()))?;
    let sisx = cwd.join("build").join(format!("{}.sisx", m.package.name));
    if !sisx.is_file() {
        return Err(Error::Other(format!(
            "SISX not found: build/{}.sisx (run symdev package; for a Rust project, cargo build)",
            m.package.name
        )));
    }
    Ok(ExeTarget::installed(sisx, uid3))
}

#[cfg(test)]
mod tests;
