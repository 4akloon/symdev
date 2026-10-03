use clap::{Parser, Subcommand, ValueEnum};
use symdev_manifest::Device;
use symdev_sdk::{PackageId, Pins};

#[derive(Parser)]
#[command(name = "symdev", disable_help_subcommand = true)]
pub struct Cli {
    /// Never download: a missing toolchain package is an error naming the command that
    /// installs it.
    #[arg(long, global = true)]
    pub offline: bool,
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    New {
        #[arg(value_parser = package_name)]
        name: String,
        /// The only device profile today.
        #[arg(long, default_value = "nokia-e52")]
        target: Target,
        /// `cpp` (an MMP project) or `rust` (a Cargo project on the Rust SDK).
        #[arg(long, alias = "language", default_value = "cpp")]
        lang: Lang,
        /// `console` (econs text app) or `gui` (Avkon app with resources).
        #[arg(long, default_value = "console")]
        template: Template,
    },
    Build,
    Package,
    Deploy,
    /// Install build/<name>.sisx into EKA2L1 and launch it (SYMDEV_EKA2L1).
    Run,
    /// Run the application in EKA2L1 and report what it wrote to
    /// `E:\symdev\results\<uid3>.json` (design spec §11).
    Test {
        /// The only test backend today: run in EKA2L1 and read the result file back.
        #[arg(long)]
        emulator: bool,
    },
    /// Append the DLLs' new exports to their frozen .def files (eabi/<name>u.def).
    Freeze,
    /// Make the `symdev-ld` and `symdev-rustc` links cargo starts (design spec §4).
    SetupLinker {
        /// Where to put them; default: beside this symdev.
        #[arg(long)]
        dir: Option<std::path::PathBuf>,
    },
    /// List the running emulators symdev started and the emulator profiles.
    Devices,
    /// Start or stop an emulator of symdev's own (design spec §5).
    Emulator {
        #[command(subcommand)]
        action: EmulatorAction,
    },
    /// Manage the toolchain packages (GCCE, platform SDK) under SYMDEV_HOME, from the
    /// sources listed in ~/.config/symdev/sources.toml.
    Sdk {
        #[command(subcommand)]
        action: SdkAction,
    },
}

#[derive(Subcommand)]
pub enum EmulatorAction {
    /// Start an EKA2L1 on a profile (e.g. rm-469); prints its id.
    Start { profile: String },
    /// Stop an emulator symdev started (e.g. emulator-1).
    Stop { id: String },
}

#[derive(Subcommand)]
pub enum SdkAction {
    /// List the installed packages and those the sources offer.
    List,
    /// Install packages, e.g. 'gcce;12.1.0' (quote the `;`); with none, what the project
    /// in the current directory needs.
    Install {
        #[arg(value_parser = package_id)]
        ids: Vec<PackageId>,
    },
    /// Remove installed packages.
    Uninstall {
        #[arg(required = true, value_parser = package_id)]
        ids: Vec<PackageId>,
    },
}

#[derive(Clone, ValueEnum)]
pub enum Target {
    #[value(name = "nokia-e52")]
    NokiaE52,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum Template {
    Console,
    Gui,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum Lang {
    Cpp,
    Rust,
}

fn package_name(s: &str) -> Result<String, String> {
    let mut chars = s.chars();
    let valid = match chars.next() {
        Some(c) if c.is_ascii_alphabetic() => chars.all(|c| c.is_ascii_alphanumeric() || c == '_'),
        _ => false,
    };
    if valid {
        Ok(s.to_owned())
    } else {
        Err(format!("invalid name `{s}`"))
    }
}

/// A package id from the command line. One without a `;` is most likely what the shell
/// left of an unquoted id: `symdev sdk install gcce;12.1.0` runs `symdev sdk install gcce`
/// and then `12.1.0`, so the error shows the quoted form (the pinned id of that kind).
fn package_id(s: &str) -> Result<PackageId, String> {
    PackageId::parse(s).map_err(|e| {
        if s.is_empty() || s.contains(';') {
            return e.to_string();
        }
        let pins = [
            Pins::gcce(),
            Pins::platform_sdk(Device::NokiaE52),
            Pins::rust_sdk(),
        ];
        let example = pins.iter().find(|id| id.kind() == s).unwrap_or(&pins[0]);
        format!(
            "{e}; quote the id: {} (unquoted, the shell ends the command at the `;`)",
            example.shell_word()
        )
    })
}
