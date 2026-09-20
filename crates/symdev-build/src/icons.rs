use std::path::{Path, PathBuf};

use symdev_core::{Artifact, Error, Project, Result};

use crate::resources::AppTarget;

/// The app's scalable icon (`[symbian] icon`): one SVG built into `<app>_aif.mif`, the
/// file the SDK examples name in `LOCALISABLE_APP_INFO.icon_file`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppIcon {
    pub source: PathBuf,
    /// The EXE MMP's name.
    pub app: String,
}

impl AppIcon {
    /// `source` relative to the project root; the app is the project's first EXE MMP.
    pub fn of(project: &Project, source: &Path) -> Result<Self> {
        let is_svg = source
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e.eq_ignore_ascii_case("svg"));
        if !is_svg {
            return Err(Error::Other(format!(
                "TODO: icon {} is not an SVG (bitmap icons not observed)",
                source.display()
            )));
        }
        let app = AppTarget::from_mmps(project)
            .map_err(|e| {
                Error::Other(format!("icon set but no application to name it after: {e}"))
            })?
            .name()
            .to_string();
        Ok(Self {
            source: project.root.join(source),
            app,
        })
    }

    pub fn mif(&self, build_dir: &Path) -> PathBuf {
        build_dir.join(format!("{}_aif.mif", self.app))
    }

    /// `mifconv /H` header: icon enum for C++ that draws the icon itself.
    pub fn mbg(&self, build_dir: &Path) -> PathBuf {
        build_dir.join(format!("{}_aif.mbg", self.app))
    }

    pub fn install_dest(&self) -> String {
        format!("!:\\resource\\apps\\{}_aif.mif", self.app)
    }

    pub fn artifact(&self, build_dir: &Path) -> Artifact {
        Artifact::installed(self.mif(build_dir), self.install_dest())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icon_installs_where_the_sdk_examples_point_icon_file() {
        let icon = AppIcon {
            source: PathBuf::from("/p/gfx/gui.svg"),
            app: "gui".into(),
        };
        assert_eq!(icon.install_dest(), "!:\\resource\\apps\\gui_aif.mif");
        assert_eq!(
            icon.mif(Path::new("/p/build")),
            Path::new("/p/build/gui_aif.mif")
        );
    }
}
