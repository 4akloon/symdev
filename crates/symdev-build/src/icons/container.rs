//! What one `[[icons]]` container writes to `build/` and installs (experiment 64).
use std::path::{Path, PathBuf};

use symdev_core::Artifact;
use symdev_manifest::IconContainer;

/// The files of one container: the `.mif` always, the `.mbm` when a source is a
/// bitmap — `mifconv` keeps bitmaps in that sibling and points the `.mif` at it — and
/// the `.mbg` when a header was asked for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IconOutputs {
    mif: PathBuf,
    mbm: Option<PathBuf>,
    mbg: Option<PathBuf>,
    dest: String,
}

impl IconOutputs {
    pub fn of(container: &IconContainer, build_dir: &Path) -> Self {
        let name = container.mif_name();
        let has_bitmaps = container.sources.iter().any(|s| Self::is_bitmap(&s.file));
        Self {
            mif: build_dir.join(name),
            mbm: has_bitmaps.then(|| build_dir.join(Self::mbm_name(name))),
            mbg: container.header.as_ref().map(|h| build_dir.join(h)),
            dest: container.dest.clone(),
        }
    }

    pub fn is_bitmap(file: &Path) -> bool {
        file.extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e.eq_ignore_ascii_case("bmp"))
    }

    /// `games.mif` → `games.mbm` (the manifest guarantees the `.mif` extension).
    fn mbm_name(mif: &str) -> String {
        let stem = mif.get(..mif.len().saturating_sub(4)).unwrap_or(mif);
        format!("{stem}.mbm")
    }

    pub fn mif(&self) -> &Path {
        &self.mif
    }

    pub fn mbm(&self) -> Option<&Path> {
        self.mbm.as_deref()
    }

    pub fn mbg(&self) -> Option<&Path> {
        self.mbg.as_deref()
    }

    /// The `.mif` at its destination, then the `.mbm` next to it.
    pub fn artifacts(&self) -> Vec<Artifact> {
        let mut out = vec![Artifact::installed(self.mif.clone(), self.dest.clone())];
        if let Some(mbm) = &self.mbm {
            let dir = self.dest.rsplit_once('\\').map_or("", |(dir, _)| dir);
            let name = Self::mbm_name(self.dest.rsplit('\\').next().unwrap_or(""));
            out.push(Artifact::installed(mbm.clone(), format!("{dir}\\{name}")));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use symdev_manifest::IconSource;

    fn container(sources: &[&str], header: Option<&str>) -> IconContainer {
        IconContainer {
            dest: "!:\\resource\\apps\\0xa000ef77\\games.mif".into(),
            header: header.map(str::to_string),
            sources: sources
                .iter()
                .map(|f| IconSource {
                    file: PathBuf::from(f),
                    depth: "c24".into(),
                    animated: false,
                })
                .collect(),
        }
    }

    #[test]
    fn a_bitmap_source_adds_the_mbm_next_to_the_mif() {
        let out = IconOutputs::of(
            &container(&["gfx/a.bmp", "gfx/b.svg"], Some("puzzles.mbg")),
            Path::new("/p/build"),
        );
        assert_eq!(out.mif(), Path::new("/p/build/games.mif"));
        assert_eq!(out.mbm(), Some(Path::new("/p/build/games.mbm")));
        assert_eq!(out.mbg(), Some(Path::new("/p/build/puzzles.mbg")));
        let artifacts = out.artifacts();
        assert_eq!(
            artifacts[0],
            Artifact::installed(
                "/p/build/games.mif",
                "!:\\resource\\apps\\0xa000ef77\\games.mif"
            )
        );
        assert_eq!(
            artifacts[1],
            Artifact::installed(
                "/p/build/games.mbm",
                "!:\\resource\\apps\\0xa000ef77\\games.mbm"
            )
        );
    }

    #[test]
    fn an_svg_only_container_is_the_mif_alone() {
        let out = IconOutputs::of(&container(&["gfx/app.svg"], None), Path::new("/p/build"));
        assert_eq!(out.mbm(), None);
        assert_eq!(out.mbg(), None);
        assert_eq!(out.artifacts().len(), 1);
    }
}
