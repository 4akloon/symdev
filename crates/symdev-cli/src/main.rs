mod cli;

use std::path::Path;
use std::process::ExitCode;

use clap::{CommandFactory, Parser};

use cli::{Cli, Commands};

fn main() -> ExitCode {
    match Cli::parse().command {
        None => {
            let _ = Cli::command().print_help();
            ExitCode::SUCCESS
        }
        Some(Commands::New { .. }) => not_implemented("symdev new", "M4"),
        Some(Commands::Build) => require_manifest("symdev build", "M1"),
        Some(Commands::Package) => require_manifest("symdev package", "M2"),
        Some(Commands::Deploy) => require_manifest("symdev deploy", "M4"),
    }
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
    let err = symdev_core::Error::NotImplemented { feature, milestone };
    eprintln!("error: {err}");
    ExitCode::from(1)
}
