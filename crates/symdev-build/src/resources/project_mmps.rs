//! `ProjectMmps`: the MMPs a project's `bld.inf` lists, preprocessed and parsed.
use std::path::{Path, PathBuf};

use symdev_core::{Error, Project, Result};

use crate::project::{HostPath, ProjectCpp, ProjectPass};
use crate::{BldInf, Mmp};

/// The MMPs a project's `bld.inf` lists, parsed, with the directory each lives in.
pub(crate) struct ProjectMmps {
    pub(crate) mmps: Vec<(PathBuf, Mmp)>,
    pub(crate) bld: BldInf,
    /// The directory the `bld.inf` lives in; every path in it is relative to this.
    pub(crate) bld_dir: PathBuf,
}

impl ProjectMmps {
    pub(crate) fn load(project: &Project, epocroot: &Path) -> Result<Self> {
        let group = project.root.join("group").join("bld.inf");
        let root = project.root.join("bld.inf");
        let bld_path = if group.is_file() {
            group
        } else if root.is_file() {
            root
        } else {
            return Err(Error::Other("no bld.inf".into()));
        };
        let cpp = ProjectCpp::new(epocroot);
        let bld = BldInf::parse(
            &cpp.run(&bld_path, ProjectPass::Platform)?,
            &cpp.run(&bld_path, ProjectPass::Gcce)?,
        )
        .map_err(|e| Error::Other(e.to_string()))?;
        if bld.mmp_files.is_empty() {
            return Err(Error::Other("no MMP to build".into()));
        }
        let bld_dir = bld_path.parent().unwrap_or(&project.root).to_path_buf();
        let mut mmps = Vec::new();
        for rel in &bld.mmp_files {
            let path = Self::find(&bld_dir, rel)?;
            let text = cpp.run(&path, ProjectPass::Gcce)?;
            let mmp = Mmp::parse(&text).map_err(|e| Error::Other(e.to_string()))?;
            let dir = path.parent().unwrap_or(&bld_dir).to_path_buf();
            mmps.push((dir, mmp));
        }
        Ok(Self { mmps, bld, bld_dir })
    }

    /// The `.mmp` a `PRJ_MMPFILES` line names: `.mmp` supplied when missing (§4.4), and
    /// the spelling matched case-insensitively as the SDK's filesystem would (§2.4).
    fn find(bld_dir: &Path, rel: &Path) -> Result<PathBuf> {
        let named = rel.to_string_lossy().replace('\\', "/");
        let with_ext = if Path::new(&named)
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("mmp"))
        {
            named.clone()
        } else {
            format!("{named}.mmp")
        };
        HostPath::find(bld_dir, &with_ext).ok_or_else(|| {
            Error::Other(format!(
                "{}: the MMP {named} is not in {}",
                bld_dir.join("bld.inf").display(),
                bld_dir.display()
            ))
        })
    }
}
