//! What `symdev package` carries: the build's outputs plus the manifest's own entries.
use std::path::Path;

use symdev_build::{AppIcon, AppTarget, BuildOutputs, IconOutputs};
use symdev_core::{Artifact, Error, Project};

/// The EXE, the resources the project's MMPs compile (`BuildOutputs`), the icon, the
/// `[[icons]]` containers and the manifest's `[[install]]` files; a project without
/// `bld.inf` packages its EXE, its containers and its `[[install]]` files alone.
pub(crate) fn package_artifacts(
    project: &Project,
    e32: &Path,
    icon: Option<&Path>,
    icons: &[symdev_manifest::IconContainer],
    install: &[symdev_manifest::InstallFile],
    epocroot: &Path,
) -> Result<Vec<Artifact>, Error> {
    let cwd = &project.root;
    let mut outputs = if AppTarget::has_bld_inf(project) {
        let mut outputs = BuildOutputs::of(project, epocroot)?;
        if let Some(source) = icon {
            outputs.push(AppIcon::of(project, source, epocroot)?.artifact(&cwd.join("build")));
        }
        outputs
            .into_iter()
            .filter(|a| a.dest.is_some() || a.path == cwd.join(e32))
            .collect()
    } else {
        vec![Artifact::exe(cwd.join(e32))]
    };
    for container in icons {
        outputs.extend(IconOutputs::of(container, &cwd.join("build")).artifacts());
    }
    for file in install {
        outputs.push(Artifact::installed(
            cwd.join(&file.source),
            file.dest.clone(),
        ));
    }
    for a in &outputs {
        if a.dest.is_some() && !a.path.is_file() {
            return Err(Error::Other(format!(
                "file to install not found: {} (run symdev build)",
                a.path.display()
            )));
        }
    }
    Ok(outputs)
}
