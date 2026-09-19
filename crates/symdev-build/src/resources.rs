use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use symdev_core::{Artifact, Error, Project, Result};

use crate::{BldInf, Mmp, MmpResource};

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

/// What `symdev build` writes to `build/` and `symdev package` installs: each MMP's EXE,
/// then its compiled resources with their install destinations.
pub struct BuildOutputs;

impl BuildOutputs {
    pub fn of(project: &Project) -> Result<Vec<Artifact>> {
        let build_dir = project.root.join("build");
        let mut out = Vec::new();
        for (_, mmp) in ProjectMmps::load(project)?.mmps {
            out.push(Artifact::exe(build_dir.join(format!("{}.exe", mmp.name()))));
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

impl Mmp {
    /// `TARGET` without its extension.
    pub fn name(&self) -> &str {
        Path::new(&self.target)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(self.target.as_str())
    }

    /// `LIBRARY` entries as the `.dso` import libraries GCCE links (`euser.lib` →
    /// `euser.dso`).
    pub fn dso_libraries(&self) -> Vec<String> {
        self.library
            .iter()
            .map(|lib| match lib.rsplit_once('.') {
                Some((stem, ext)) if ext.eq_ignore_ascii_case("lib") => format!("{stem}.dso"),
                _ => lib.clone(),
            })
            .collect()
    }
}

impl MmpResource {
    pub fn stem(&self) -> Result<&str> {
        Path::new(&self.file)
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| Error::Other(format!("bad resource name {}", self.file)))
    }

    /// `!:` install path of the compiled `.rsc`. Registration resources go to the import
    /// directory the SDK example `.pkg` uses; others to their `TARGETPATH`.
    pub fn install_dest(&self) -> Result<String> {
        let stem = self.stem()?;
        if stem.to_ascii_lowercase().ends_with("_reg") {
            return Ok(format!("!:\\private\\10003a3f\\import\\apps\\{stem}.rsc"));
        }
        let dir = self.targetpath.as_deref().ok_or_else(|| {
            Error::Other(format!("START RESOURCE {} needs TARGETPATH", self.file))
        })?;
        let dir = dir.replace('/', "\\");
        let dir = dir.trim_end_matches('\\');
        let dir = if dir.starts_with('\\') {
            dir.to_string()
        } else {
            format!("\\{dir}")
        };
        Ok(format!("!:{dir}\\{stem}.rsc"))
    }

    /// The `.rss` file: relative to the block's `SOURCEPATH`, else the MMP directory.
    pub fn source(&self, mmp_dir: &Path) -> PathBuf {
        let rel = |p: &str| p.replace('\\', "/");
        match &self.sourcepath {
            Some(sp) if Path::new(&rel(sp)).is_absolute() => {
                Path::new(&rel(sp)).join(rel(&self.file))
            }
            Some(sp) => mmp_dir.join(rel(sp)).join(rel(&self.file)),
            None => mmp_dir.join(rel(&self.file)),
        }
    }
}

/// Case-insensitive view of `epoc32/include` for a case-sensitive host: the SDK was
/// written on Windows, so headers include each other with the wrong case
/// (`fbs.h` → `FbsMessage.h`, file `fbsmessage.h`). One symlink per mismatched
/// include name, pointing at the real file.
pub struct SdkIncludeCaseFold;

impl SdkIncludeCaseFold {
    const MARKER: &str = ".symdev-casefold";

    /// Build the overlay in `out` once; returns `out`.
    pub fn ensure(include: &Path, out: &Path) -> Result<PathBuf> {
        if out.join(Self::MARKER).is_file() {
            return Ok(out.to_path_buf());
        }
        let mut by_lower: BTreeMap<String, PathBuf> = BTreeMap::new();
        let mut names: BTreeSet<String> = BTreeSet::new();
        let mut stack = vec![include.to_path_buf()];
        while let Some(dir) = stack.pop() {
            let entries =
                std::fs::read_dir(&dir).map_err(|e| Error::Other(format!("read {dir:?}: {e}")))?;
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                    continue;
                }
                let Ok(rel) = path.strip_prefix(include) else {
                    continue;
                };
                let rel = rel.to_string_lossy().replace('\\', "/");
                by_lower
                    .entry(rel.to_lowercase())
                    .or_insert_with(|| path.clone());
                if let Ok(bytes) = std::fs::read(&path) {
                    Self::include_names(&bytes, &mut names);
                }
            }
        }
        std::fs::create_dir_all(out).map_err(|e| Error::Other(format!("{out:?}: {e}")))?;
        for name in names {
            if include.join(&name).exists() {
                continue;
            }
            let Some(real) = by_lower.get(&name.to_lowercase()) else {
                continue;
            };
            let link = out.join(&name);
            if let Some(parent) = link.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| Error::Other(format!("{parent:?}: {e}")))?;
            }
            if link.symlink_metadata().is_err() {
                Self::symlink(real, &link)?;
            }
        }
        std::fs::write(out.join(Self::MARKER), include.display().to_string())
            .map_err(|e| Error::Other(format!("{out:?}: {e}")))?;
        Ok(out.to_path_buf())
    }

    fn include_names(bytes: &[u8], names: &mut BTreeSet<String>) {
        for line in bytes.split(|&b| b == b'\n') {
            let line = String::from_utf8_lossy(line);
            let t = line.trim_start();
            let Some(rest) = t.strip_prefix('#') else {
                continue;
            };
            let Some(rest) = rest.trim_start().strip_prefix("include") else {
                continue;
            };
            let rest = rest.trim_start();
            let close = match rest.chars().next() {
                Some('<') => '>',
                Some('"') => '"',
                _ => continue,
            };
            if let Some(end) = rest[1..].find(close) {
                let name = rest[1..1 + end].replace('\\', "/");
                if !name.is_empty() && !name.starts_with('/') && !name.contains("..") {
                    names.insert(name);
                }
            }
        }
    }

    #[cfg(unix)]
    fn symlink(real: &Path, link: &Path) -> Result<()> {
        std::os::unix::fs::symlink(real, link)
            .map_err(|e| Error::Other(format!("symlink {link:?}: {e}")))
    }

    #[cfg(not(unix))]
    fn symlink(_real: &Path, _link: &Path) -> Result<()> {
        Ok(())
    }
}

fn read(path: &Path) -> Result<String> {
    std::fs::read_to_string(path).map_err(|e| Error::Other(format!("read {path:?}: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn res(file: &str, targetpath: Option<&str>) -> MmpResource {
        MmpResource {
            file: file.into(),
            sourcepath: Some("..\\data".into()),
            targetpath: targetpath.map(Into::into),
            header: true,
            lang: Vec::new(),
        }
    }

    #[test]
    fn app_resource_installs_to_its_targetpath() {
        assert_eq!(
            res("gui.rss", Some("\\resource\\apps"))
                .install_dest()
                .unwrap(),
            "!:\\resource\\apps\\gui.rsc"
        );
    }

    #[test]
    fn reg_resource_installs_to_import_apps_like_the_sdk_pkg() {
        assert_eq!(
            res("gui_reg.rss", Some("\\private\\10003a3f\\apps"))
                .install_dest()
                .unwrap(),
            "!:\\private\\10003a3f\\import\\apps\\gui_reg.rsc"
        );
    }

    #[test]
    fn resource_source_follows_sourcepath() {
        assert_eq!(
            res("gui.rss", None).source(Path::new("/p/group")),
            PathBuf::from("/p/group/../data/gui.rss")
        );
    }

    #[test]
    fn library_names_become_dso() {
        let mut m = Mmp::parse("TARGET gui.exe\nTARGETTYPE EXE\nSOURCE a.cpp\n").unwrap();
        m.library = vec!["euser.lib".into(), "avkon.lib".into(), "x.dso".into()];
        assert_eq!(m.dso_libraries(), ["euser.dso", "avkon.dso", "x.dso"]);
    }

    #[test]
    fn casefold_links_wrong_case_includes() {
        let dir = std::env::temp_dir().join(format!("symdev-casefold-{}", std::process::id()));
        let inc = dir.join("include");
        std::fs::create_dir_all(inc.join("sub")).unwrap();
        std::fs::write(
            inc.join("fbs.h"),
            "#include <FbsMessage.h>\n#include \"Sub/Deep.h\"\n",
        )
        .unwrap();
        std::fs::write(inc.join("fbsmessage.h"), "x").unwrap();
        std::fs::write(inc.join("sub").join("deep.h"), "y").unwrap();
        let out = SdkIncludeCaseFold::ensure(&inc, &dir.join("overlay")).unwrap();
        assert_eq!(
            std::fs::read_to_string(out.join("FbsMessage.h")).unwrap(),
            "x"
        );
        assert_eq!(
            std::fs::read_to_string(out.join("Sub/Deep.h")).unwrap(),
            "y"
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
