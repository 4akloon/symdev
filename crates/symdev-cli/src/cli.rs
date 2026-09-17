use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[command(name = "symdev")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    New {
        name: String,
        #[arg(long)]
        target: Target,
        #[arg(long, default_value = "cpp")]
        lang: Lang,
    },
    Build,
    Package,
    Deploy,
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
