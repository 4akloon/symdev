//! `BuildOutputs`: what `symdev build` writes and `symdev package` installs.
use std::path::Path;

use symdev_core::{Artifact, Project, Result};

use super::project_mmps::ProjectMmps;

/// What `symdev build` writes to `build/` and `symdev package` installs: each MMP's EXE,
/// then its compiled resources with their install destinations.
pub struct BuildOutputs;

impl BuildOutputs {
    pub fn of(project: &Project, epocroot: &Path) -> Result<Vec<Artifact>> {
        let build_dir = project.root.join("build");
        let mut out = Vec::new();
        for (_, mmp) in ProjectMmps::load(project, epocroot)?.mmps {
            if mmp.target_type.eq_ignore_ascii_case("DLL") {
                out.push(Artifact::installed(
                    build_dir.join(format!("{}.dll", mmp.name())),
                    format!("!:\\sys\\bin\\{}.dll", mmp.name()),
                ));
            } else {
                out.push(Artifact::exe(build_dir.join(format!("{}.exe", mmp.name()))));
            }
            for res in &mmp.resource {
                out.push(Artifact::installed(
                    build_dir.join(format!("{}.rsc", res.stem()?)),
                    res.install_dest()?,
                ));
            }
        }
        Ok(out)
    }
}
