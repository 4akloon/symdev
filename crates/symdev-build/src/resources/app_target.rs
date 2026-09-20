//! `AppTarget`: the name of the binary the package installs and launches.
use symdev_core::{Error, Project, Result};

use super::project_mmps::ProjectMmps;

/// The application binary's name: the `TARGET` of the project's first EXE MMP, without
/// its extension. `symdev build` writes `build/<AppTarget>.exe`, so packaging, the
/// registration resource and the icon all have to use the same name — the manifest
/// `package.name` is the package's identity, not the binary's (`third-party-app-puzzles`
/// gap 11: the two disagree whenever the MMP target is not the manifest name).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppTarget(String);

impl AppTarget {
    /// The first EXE MMP's name; a project with no `bld.inf` falls back to `package_name`.
    pub fn of(project: &Project, package_name: &str) -> Result<Self> {
        if !Self::has_bld_inf(project) {
            return Ok(Self(package_name.to_string()));
        }
        Self::from_mmps(project)
    }

    /// The first EXE MMP's name; an error when the project has no MMP that builds one.
    pub fn from_mmps(project: &Project) -> Result<Self> {
        let name = ProjectMmps::load(project)?
            .mmps
            .into_iter()
            .map(|(_, mmp)| mmp)
            .find(|mmp| mmp.target_type.eq_ignore_ascii_case("EXE"))
            .ok_or_else(|| Error::Other("the project has no EXE MMP".into()))?
            .name()
            .to_string();
        Ok(Self(name))
    }

    pub fn has_bld_inf(project: &Project) -> bool {
        project.root.join("group/bld.inf").is_file() || project.root.join("bld.inf").is_file()
    }

    pub fn name(&self) -> &str {
        &self.0
    }

    /// `build/<name>.exe`, the E32 image `symdev build` writes.
    pub fn exe_file(&self) -> String {
        format!("{}.exe", self.0)
    }
}

impl std::fmt::Display for AppTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
