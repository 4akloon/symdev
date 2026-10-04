//! `symdev setup-linker`: the `symdev-ld` and `symdev-rustc` links cargo starts (design
//! spec §4). `install.sh` makes them for an installed symdev; this makes them for a build
//! of a checkout.
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use symdev_core::{Error, Result};

/// The names the binary answers to besides `symdev` (`Role::of`).
const ROLES: [&str; 2] = ["symdev-ld", "symdev-rustc"];

pub(crate) fn setup_linker(dir: Option<PathBuf>) -> Result<ExitCode> {
    let exe = std::env::current_exe()
        .map_err(|e| Error::Other(format!("setup-linker: where is this symdev? {e}")))?;
    let dir = match dir {
        Some(dir) => dir,
        None => exe
            .parent()
            .map(Path::to_path_buf)
            .ok_or_else(|| Error::Other(format!("{}: no directory", exe.display())))?,
    };
    for role in ROLES {
        let link = dir.join(role);
        match std::fs::read_link(&link) {
            Ok(target) if target == exe => {}
            Ok(_) | Err(_) if link.symlink_metadata().is_ok() => {
                return Err(Error::Other(format!(
                    "setup-linker: {} exists and is not a link to {}; remove it or pass \
                     --dir",
                    link.display(),
                    exe.display()
                )));
            }
            _ => std::os::unix::fs::symlink(&exe, &link)
                .map_err(|e| Error::Other(format!("setup-linker: link {}: {e}", link.display())))?,
        }
        println!("{}", link.display());
    }
    let on_path =
        std::env::var_os("PATH").is_some_and(|path| std::env::split_paths(&path).any(|p| p == dir));
    if !on_path {
        println!(
            "note: {} is not on PATH; cargo looks the linker up there",
            dir.display()
        );
    }
    Ok(ExitCode::SUCCESS)
}
