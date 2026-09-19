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
        #[arg(long)]
        target: Target,
        #[arg(long, default_value = "cpp")]
        lang: Lang,
    },
    Build,
    Package,
    Deploy,
    /// Install build/<name>.sisx into EKA2L1 and launch it (SYMDEV_EKA2L1).
    Run,
}

#[derive(Clone, ValueEnum)]
pub enum Target {
    #[value(name = "nokia-e52")]
    NokiaE52,
}

#[derive(Clone, ValueEnum)]
pub enum Lang {
    Cpp,
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
