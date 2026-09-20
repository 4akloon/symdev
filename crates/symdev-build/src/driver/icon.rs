//! `GcceBuild`: the app icon, encoded natively (experiment 55, svgb-mif-spec.md).

use std::path::Path;

use symdev_core::{Error, Result};
use symdev_mif::{MifFile, MifIcon, SvgElement, Svgb};

use super::GcceBuild;
use crate::icons::AppIcon;

impl GcceBuild {
    /// `<app>_aif.mif` (and its `.mbg`) from the SVG, without Wine.
    pub(super) fn compile_icon(&self, icon: &AppIcon, build_dir: &Path) -> Result<()> {
        let source = std::fs::read_to_string(&icon.source)
            .map_err(|e| Error::Other(format!("read icon {}: {e}", icon.source.display())))?;
        let document = SvgElement::parse(&source)
            .map_err(|e| Error::Other(format!("{}: {e}", icon.source.display())))?;
        let svgb = Svgb::new(Svgb::MIFCONV_VERSION)?
            .encode(&document)
            .map_err(|e| Error::Other(format!("{}: {e}", icon.source.display())))?;
        let name = icon
            .source
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("icon.svg");
        let mif = MifFile::new(vec![MifIcon::svg(name, svgb)]);
        let mif_path = icon.mif(build_dir);
        std::fs::write(&mif_path, mif.bytes()?)
            .map_err(|e| Error::Other(format!("write {}: {e}", mif_path.display())))?;
        let mbg_path = icon.mbg(build_dir);
        let mif_name = mif_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("icon.mif");
        std::fs::write(&mbg_path, mif.mbg_text(mif_name))
            .map_err(|e| Error::Other(format!("write {}: {e}", mbg_path.display())))?;
        Ok(())
    }
}
