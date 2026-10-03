//! The `symdev-ld` role: cargo's linker for `arm-symbian-e32` (design spec §4).
mod cargo_link_env;
mod cargo_output;
mod link_kind;
mod linker_args;

pub(crate) use cargo_link_env::CargoLinkEnv;
pub(crate) use cargo_output::CargoOutput;
pub(crate) use link_kind::LinkKind;
pub(crate) use linker_args::LinkerArgs;

#[cfg(test)]
mod tests;
