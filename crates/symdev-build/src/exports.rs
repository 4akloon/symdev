use std::path::{Component, Path, PathBuf};

use symdev_core::{Error, Project, Result};
use symdev_elf2e32::E32DefFile;

use crate::Mmp;
use crate::resources::ProjectMmps;

impl Mmp {
    pub fn is_dll(&self) -> bool {
        self.target_type.eq_ignore_ascii_case("DLL")
    }

    /// Where the frozen `.def` lives (`mmp.pm`): the `DEFFILE` directory, or `../eabi`
    /// next to the MMP when `DEFFILE` has none (`~` in a `DEFFILE` path also means
    /// `eabi`); the `DEFFILE` or `TARGET` base name plus `u` unless `NOSTRICTDEF`;
    /// `.def` unless `DEFFILE` names another extension. The SDK examples ship these in
    /// another case than the MMP spells them (`OriginalDll.def` → `originaldllu.def`), so
    /// an existing file is found case-insensitively.
    pub fn frozen_def(&self, mmp_dir: &Path) -> Result<PathBuf> {
        let spec = self.deffile.as_deref().map(|d| d.replace('\\', "/"));
        let spec_path = spec.as_deref().map(Path::new);
        let dir = match spec_path.and_then(Path::parent) {
            Some(parent) if !parent.as_os_str().is_empty() => {
                if parent.is_absolute() {
                    return Err(Error::Other(format!(
                        "TODO: absolute DEFFILE {} (EPOCROOT-relative, not observed)",
                        spec.as_deref().unwrap_or_default()
                    )));
                }
                let mut dir = mmp_dir.to_path_buf();
                for part in parent.components() {
                    match part {
                        Component::Normal(p) if p == "~" => dir.push("eabi"),
                        Component::CurDir => {}
                        other => dir.push(other),
                    }
                }
                dir
            }
            _ => mmp_dir.join("..").join("eabi"),
        };
        let base = spec_path
            .and_then(Path::file_stem)
            .and_then(|s| s.to_str())
            .unwrap_or(self.name());
        let ext = spec_path
            .and_then(Path::extension)
            .and_then(|s| s.to_str())
            .unwrap_or("def");
        let suffix = if self.nostrictdef { "" } else { "u" };
        let file = format!("{base}{suffix}.{ext}");
        let dir = Self::lexical(&dir);
        Ok(Self::find_case_insensitive(&dir, &file).unwrap_or_else(|| dir.join(file)))
    }

    /// `a/group/../eabi` → `a/eabi`, without touching the filesystem.
    fn lexical(path: &Path) -> PathBuf {
        let mut out = PathBuf::new();
        for part in path.components() {
            match part {
                Component::ParentDir
                    if matches!(out.components().next_back(), Some(Component::Normal(_))) =>
                {
                    out.pop();
                }
                Component::CurDir => {}
                other => out.push(other),
            }
        }
        out
    }

    fn find_case_insensitive(dir: &Path, file: &str) -> Option<PathBuf> {
        let exact = dir.join(file);
        if exact.is_file() {
            return Some(exact);
        }
        std::fs::read_dir(dir)
            .ok()?
            .flatten()
            .map(|e| e.path())
            .find(|p| {
                p.is_file()
                    && p.file_name()
                        .and_then(|n| n.to_str())
                        .is_some_and(|n| n.eq_ignore_ascii_case(file))
            })
    }
}

/// A DLL's exports as `symdev build` left them in `build/<name>.def`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DllExports {
    pub dll: String,
    /// The frozen `.def` (existing or where `symdev freeze` will write it).
    pub frozen_def: PathBuf,
    /// Exports the frozen `.def` does not list yet.
    pub unfrozen: Vec<String>,
}

/// Frozen exports of a project's DLLs: which are not yet frozen, and `symdev freeze`.
pub struct FrozenExports;

impl FrozenExports {
    /// Every DLL MMP with the `--defoutput` of its last build.
    pub fn of(project: &Project) -> Result<Vec<DllExports>> {
        let build_dir = project.root.join("build");
        let mut out = Vec::new();
        for (mmp_dir, mmp) in ProjectMmps::load(project)?.mmps {
            if !mmp.is_dll() {
                continue;
            }
            let generated_path = build_dir.join(format!("{}.def", mmp.name()));
            let generated = std::fs::read_to_string(&generated_path).map_err(|e| {
                Error::Other(format!(
                    "read {} (run symdev build): {e}",
                    generated_path.display()
                ))
            })?;
            out.push(DllExports {
                dll: mmp.name().to_string(),
                frozen_def: mmp.frozen_def(&mmp_dir)?,
                unfrozen: E32DefFile::new_names(&generated)
                    .into_iter()
                    .map(str::to_string)
                    .collect(),
            });
        }
        Ok(out)
    }

    /// Append each DLL's unfrozen exports to its frozen `.def` (creating it and its
    /// directory on first freeze). Returns the DLLs whose `.def` changed.
    pub fn freeze(project: &Project) -> Result<Vec<DllExports>> {
        let build_dir = project.root.join("build");
        let mut changed = Vec::new();
        for dll in Self::of(project)? {
            let generated = std::fs::read_to_string(build_dir.join(format!("{}.def", dll.dll)))
                .map_err(|e| Error::Other(format!("read build/{}.def: {e}", dll.dll)))?;
            let existing = if dll.frozen_def.is_file() {
                Some(
                    std::fs::read_to_string(&dll.frozen_def)
                        .map_err(|e| Error::Other(format!("read {:?}: {e}", dll.frozen_def)))?,
                )
            } else {
                None
            };
            let Some(text) = E32DefFile::freeze(existing.as_deref(), &generated)? else {
                continue;
            };
            if let Some(dir) = dll.frozen_def.parent() {
                std::fs::create_dir_all(dir)
                    .map_err(|e| Error::Other(format!("create {dir:?}: {e}")))?;
            }
            std::fs::write(&dll.frozen_def, text)
                .map_err(|e| Error::Other(format!("write {:?}: {e}", dll.frozen_def)))?;
            changed.push(dll);
        }
        Ok(changed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dll(extra: &str) -> Mmp {
        Mmp::parse(&format!(
            "TARGET mathlib.dll\nTARGETTYPE DLL\nUID 0x1000008d 0xe5d1b001\nSOURCE m.cpp\n{extra}"
        ))
        .unwrap()
    }

    #[test]
    fn frozen_def_defaults_to_eabi_next_to_the_mmp_with_u_suffix() {
        let dir = Path::new("/nonexistent/p/group");
        assert_eq!(
            dll("").frozen_def(dir).unwrap(),
            Path::new("/nonexistent/p/eabi/mathlibu.def")
        );
        // DEFFILE without a directory keeps the default directory (SDK extensionpattern).
        assert_eq!(
            dll("DEFFILE OriginalDll.def\n").frozen_def(dir).unwrap(),
            Path::new("/nonexistent/p/eabi/OriginalDllu.def")
        );
        assert_eq!(
            dll("deffile .\\POLY1ARM.def\nnostrictdef\n")
                .frozen_def(dir)
                .unwrap(),
            Path::new("/nonexistent/p/group/POLY1ARM.def")
        );
        assert_eq!(
            dll("DEFFILE ..\\~\\x.def\n").frozen_def(dir).unwrap(),
            Path::new("/nonexistent/p/eabi/xu.def")
        );
    }

    #[test]
    fn frozen_def_is_found_in_another_case() {
        let root = std::env::temp_dir().join(format!("symdev-frozen-def-{}", std::process::id()));
        let group = root.join("group");
        std::fs::create_dir_all(&group).unwrap();
        std::fs::create_dir_all(root.join("eabi")).unwrap();
        std::fs::write(root.join("eabi").join("originaldllu.def"), "EXPORTS\n").unwrap();
        assert_eq!(
            dll("DEFFILE OriginalDll.def\n").frozen_def(&group).unwrap(),
            root.join("eabi").join("originaldllu.def")
        );
        std::fs::remove_dir_all(&root).unwrap();
    }
}
