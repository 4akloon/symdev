//! The `bld.inf` / `.mmp` front end's shared parts: preprocessing, line records and
//! host path lookup ([mmp-frontend-spec.md](../../../../docs/research/mmp-frontend-spec.md)
//! §1–§3).

mod cpp;
mod host_path;
mod line;

pub use cpp::{ProjectCpp, ProjectPass};
pub use host_path::HostPath;
pub use line::ProjectLine;

#[cfg(test)]
mod tests;
