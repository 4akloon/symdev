//! `symdev sdk list|install|uninstall`: the toolchain packages under `SYMDEV_HOME`.
use std::collections::BTreeSet;
use std::process::ExitCode;

use symdev_core::Error;
use symdev_manifest::Error as ManifestError;
use symdev_sdk::{PackageId, Pins};

use crate::cli::SdkAction;
use crate::provision::Provision;

pub fn run(action: SdkAction, provision: &Provision) -> Result<ExitCode, Error> {
    match action {
        SdkAction::List => list(provision),
        SdkAction::Install { ids } => install(provision, ids),
        SdkAction::Uninstall { ids } => uninstall(provision, &ids),
    }
}

/// `installed  <id>  (<source>)` for every receipt, then `available  <id>  (<source>)`
/// for what the sources offer and is not installed (skipped with `--offline`).
fn list(provision: &Provision) -> Result<ExitCode, Error> {
    let mut stderr = std::io::stderr();
    let mut manager = provision.manager(&mut stderr)?;
    let installed = manager.home().list()?;
    for receipt in &installed {
        println!("installed  {}  ({})", receipt.id, receipt.source);
    }
    if provision.offline() {
        return Ok(ExitCode::SUCCESS);
    }
    let ids: BTreeSet<_> = installed.into_iter().map(|r| r.id).collect();
    for (source, package) in manager.available()? {
        if !ids.contains(&package.id) {
            println!("available  {}  ({source})", package.id);
        }
    }
    Ok(ExitCode::SUCCESS)
}

/// Installs `ids`, or with none what the project in the current directory needs.
fn install(provision: &Provision, ids: Vec<PackageId>) -> Result<ExitCode, Error> {
    let ids = if ids.is_empty() {
        match symdev_manifest::load("symdev.toml") {
            Ok(m) => provision.needed(m.target.device, m.language),
            Err(ManifestError::MissingFile) => {
                return Err(Error::Other(format!(
                    "`symdev sdk install` without ids installs what the project in the \
                     current directory needs, and there is no symdev.toml here; name the \
                     packages instead, e.g. `symdev sdk install {}`",
                    Pins::gcce().shell_word()
                )));
            }
            Err(e) => return Err(Error::Other(format!("invalid manifest: {e}"))),
        }
    } else {
        ids
    };
    if ids.is_empty() {
        println!("nothing to install: the SYMDEV_* variables set every toolchain path");
        return Ok(ExitCode::SUCCESS);
    }
    let mut stderr = std::io::stderr();
    for receipt in provision.manager(&mut stderr)?.ensure(&ids)? {
        println!("installed  {}  ({})", receipt.id, receipt.source);
    }
    Ok(ExitCode::SUCCESS)
}

fn uninstall(provision: &Provision, ids: &[PackageId]) -> Result<ExitCode, Error> {
    let home = provision.home()?;
    for id in ids {
        if home.uninstall(id)? {
            println!("removed  {id}");
        } else {
            eprintln!("warning: {id} is not installed");
        }
    }
    Ok(ExitCode::SUCCESS)
}
