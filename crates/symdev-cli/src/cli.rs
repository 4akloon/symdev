use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[command(name = "symdev", disable_help_subcommand = true)]
pub struct Cli {
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
    /// Append the DLLs' new exports to their frozen .def files (eabi/<name>u.def).
    Freeze,
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
