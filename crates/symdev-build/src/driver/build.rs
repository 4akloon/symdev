//! `impl BuildBackend for GcceBuild`: the whole per-MMP build pipeline.
use symdev_core::{Artifact, BuildBackend, Error, Project, RemotePath, Result};

use super::compile::CompileIncludes;
use super::gcce_compat::GcceCompat;
use super::module::Module;
use super::source::resolve_source;
use super::{GcceBuild, arg, io};
use crate::icons::AppIcon;
use crate::resources::{GeneratedCaseFold, ProjectMmps, SdkIncludeCaseFold};

impl BuildBackend for GcceBuild {
    fn build(&self, project: &Project) -> Result<Vec<Artifact>> {
        let mmps = ProjectMmps::load(project)?;
        let build_dir = project.root.join("build");
        std::fs::create_dir_all(&build_dir).map_err(io)?;
        let cwd = RemotePath::new(arg(&project.root));
        let casefold = SdkIncludeCaseFold::ensure(
            &self.tools.epocroot.join("epoc32/include"),
            &build_dir.join("sdk-include-casefold"),
        )?;
        let compat = GcceCompat::ensure(&build_dir)?;

        let mut artifacts = Vec::new();
        if let Some(source) = &self.icon {
            let icon = AppIcon::of(project, source)?;
            self.compile_icon(&icon, &build_dir)?;
            artifacts.push(icon.artifact(&build_dir));
        }
        for (mmp_dir, mmp) in &mmps.mmps {
            let name = mmp.name();
            let module = Module::of(mmp, self.uid3)?;
            // Resources first: sources include the generated `.rsg` headers.
            for res in &mmp.resource {
                self.compile_resource(res, mmp_dir, mmp, &build_dir)?;
            }
            let mut includes = CompileIncludes {
                user: vec![build_dir.clone()],
                system: Vec::new(),
                prefix: vec![compat.header().to_path_buf()],
            };
            includes.user.extend(
                mmp.userinclude
                    .iter()
                    .map(|d| self.mmp_dir_path(mmp_dir, d)),
            );
            includes.system.extend(
                mmp.systeminclude
                    .iter()
                    .map(|d| self.mmp_dir_path(mmp_dir, d)),
            );
            includes.system.push(casefold.clone());

            let mut sources = Vec::new();
            for (i, src) in mmp.source.iter().enumerate() {
                let sp = mmp.source_sourcepath.get(i).cloned().flatten();
                sources.push(resolve_source(sp.as_deref(), mmp_dir, &project.root, src)?);
            }
            // The generated `.rsg`/`.mbg` carry the resource's own spelling; the sources
            // may include them in another case (experiment: third-party-app-puzzles).
            let mut askers = sources.clone();
            askers.extend(
                mmp.userinclude
                    .iter()
                    .map(|d| self.mmp_dir_path(mmp_dir, d)),
            );
            includes.user.push(GeneratedCaseFold::ensure(
                &build_dir,
                &askers,
                &build_dir.join("generated-casefold"),
            )?);

            let mut objs = Vec::new();
            for (i, source) in sources.iter().enumerate() {
                let src = &mmp.source[i];
                let source_dir = source.parent().unwrap_or(mmp_dir);
                let stem = source
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or(src.as_str());
                let obj = build_dir.join(format!("{stem}.o"));
                let compile =
                    self.compile_args_for(&module, source_dir, &includes, source, &obj)?;
                self.run_tool(&compile, &cwd)?;
                objs.push(obj);
            }
            let obj = objs
                .first()
                .ok_or_else(|| Error::Other("no SOURCE".into()))?;
            let elf = build_dir.join(format!("{name}.elf"));
            let map = build_dir.join(format!("{name}.{}.map", module.ext()));
            let mut link = self.link_args_for(
                &module,
                name,
                obj,
                &elf,
                &map,
                &mmp.dso_libraries(),
                std::slice::from_ref(&build_dir),
            );
            if objs.len() > 1 {
                let first = arg(obj);
                if let Some(pos) = link.iter().position(|a| a == &first) {
                    for extra in objs.iter().skip(1).rev() {
                        link.insert(pos + 1, arg(extra));
                    }
                }
            }
            self.run_tool(&link, &cwd)?;
            let out = build_dir.join(format!("{name}.{}", module.ext()));
            let frozen_def = Some(mmp.frozen_def(mmp_dir)?).filter(|p| module.dll && p.is_file());
            self.run_elf2e32(
                &self.elf2e32_args_for(
                    &module,
                    name,
                    &elf,
                    &out,
                    Some(&build_dir),
                    frozen_def.as_deref(),
                ),
                &cwd,
            )?;
            artifacts.push(if module.dll {
                Artifact::installed(out, format!("!:\\sys\\bin\\{name}.dll"))
            } else {
                Artifact::exe(out)
            });
            for res in &mmp.resource {
                artifacts.push(Artifact::installed(
                    build_dir.join(format!("{}.rsc", res.stem()?)),
                    res.install_dest()?,
                ));
            }
        }
        Ok(artifacts)
    }
}
