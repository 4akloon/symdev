//! `ProjectMmps`: the MMPs a project's `bld.inf` lists, parsed.
use std::path::PathBuf;

use symdev_core::{Error, Project, Result};

use super::read;
use crate::{BldInf, Mmp};

/// The MMPs a project's `bld.inf` lists, parsed, with the directory each lives in.
pub(crate) struct ProjectMmps {
    pub(crate) mmps: Vec<(PathBuf, Mmp)>,
}

impl ProjectMmps {
    pub(crate) fn load(project: &Project) -> Result<Self> {
        let group = project.root.join("group").join("bld.inf");
        let root = project.root.join("bld.inf");
        let bld_path = if group.is_file() {
            group
        } else if root.is_file() {
            root
        } else {
            return Err(Error::Other("no bld.inf".into()));
        };
        let bld = BldInf::parse(&read(&bld_path)?).map_err(|e| Error::Other(e.to_string()))?;
        if bld.mmp_files.is_empty() {
            return Err(Error::Other("no MMP to build".into()));
        }
        let bld_dir = bld_path.parent().unwrap_or(&project.root).to_path_buf();
        let mut mmps = Vec::new();
        for rel in &bld.mmp_files {
            let path = bld_dir.join(rel);
            let mmp = Mmp::parse(&read(&path)?).map_err(|e| Error::Other(e.to_string()))?;
            let dir = path.parent().unwrap_or(&bld_dir).to_path_buf();
            mmps.push((dir, mmp));
        }
        Ok(Self { mmps })
    }
}
