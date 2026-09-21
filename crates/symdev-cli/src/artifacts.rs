//! What `symdev package` carries: the build's outputs plus the manifest's own entries.
use std::path::Path;

use symdev_build::{AppIcon, AppTarget, BuildOutputs, IconOutputs, UiResources};
use symdev_core::{Artifact, Error, Project};

/// The EXE, the resources the project's MMPs compile (`BuildOutputs`), the icon, the
/// `[[icons]]` containers and the manifest's `[[install]]` files; a project without
/// `bld.inf` packages its EXE, its containers and its `[[install]]` files alone —
/// plus, when it has a `[ui]` section, the three files `RustBuild`'s resource stage
/// generated for it. `symdev package` re-derives the list rather than being handed
/// the build's, so anything a backend produces has to be named here too.
pub(crate) fn package_artifacts(
    project: &Project,
    e32: &Path,
    icon: Option<&Path>,
    icons: &[symdev_manifest::IconContainer],
    install: &[symdev_manifest::InstallFile],
    epocroot: &Path,
    ui: Option<&UiResources>,
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
        let mut outputs = vec![Artifact::exe(cwd.join(e32))];
        if let Some(ui) = ui {
            let build = cwd.join("build");
            outputs.extend(ui.artifacts(&build));
            if let Some(source) = &ui.icon {
                outputs.push(
                    AppIcon {
                        source: source.clone(),
                        app: ui.app.clone(),
                    }
                    .artifact(&build),
                );
            }
        }
        outputs
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
