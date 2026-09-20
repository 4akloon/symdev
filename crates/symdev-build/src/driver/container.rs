//! `GcceBuild`: one `[[icons]]` container, built natively the way `mifconv` builds it
//! (svgb-mif-spec.md §6–§7, experiment 64 for bitmap sources).

use std::path::Path;

use symdev_core::{Error, Result};
use symdev_manifest::{IconContainer, IconSource};
use symdev_mbm::{BmpImage, MbmBitmap, MbmDepth, MbmFile};
use symdev_mif::{MifDepth, MifFile, MifIcon, SvgElement, Svgb};

use super::GcceBuild;
use crate::icons::IconOutputs;

/// The bitmaps of the sibling `.mbm` as they accumulate: an icon's mask is a bitmap of
/// its own, right after the icon, so the indices the `.mif` carries count both.
#[derive(Default)]
struct Bitmaps(Vec<MbmBitmap>);

impl Bitmaps {
    fn push(&mut self, path: &Path, depth: MbmDepth) -> Result<u32> {
        let bytes = std::fs::read(path)
            .map_err(|e| Error::Other(format!("read icon source {}: {e}", path.display())))?;
        let image = BmpImage::parse(&bytes)
            .map_err(|e| Error::Other(format!("{}: {e}", path.display())))?;
        self.0.push(MbmBitmap::compile(&image, depth));
        Ok(self.0.len() as u32 - 1)
    }
}

impl GcceBuild {
    /// The container's `.mif`, its `.mbm` when a source is a bitmap, and its `.mbg`
    /// when a header is asked for, into `build_dir`.
    pub(super) fn compile_icon_container(
        &self,
        container: &IconContainer,
        project_root: &Path,
        build_dir: &Path,
    ) -> Result<()> {
        let outputs = IconOutputs::of(container, build_dir);
        let mut icons = Vec::with_capacity(container.sources.len());
        let mut bitmaps = Bitmaps::default();
        for source in &container.sources {
            let path = project_root.join(&source.file);
            let depth = MifDepth::parse(&source.depth)
                .map_err(|e| Error::Other(format!("{}: {e}", path.display())))?;
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("icon")
                .to_string();
            icons.push(if IconOutputs::is_bitmap(&path) {
                Self::bitmap_icon(source, &path, name, &depth, &mut bitmaps)?
            } else {
                MifIcon::svg_at(name, Self::svgb(&path)?, &depth, source.animated)
            });
        }
        let mif = MifFile::new(icons);
        Self::write(outputs.mif(), &mif.bytes()?)?;
        if let Some(mbm) = outputs.mbm() {
            Self::write(mbm, &MbmFile::new(bitmaps.0).bytes()?)?;
        }
        if let (Some(mbg), Some(header)) = (outputs.mbg(), &container.header) {
            Self::write(mbg, mif.mbg_text(header).as_bytes())?;
        }
        Ok(())
    }

    fn svgb(path: &Path) -> Result<Vec<u8>> {
        let source = std::fs::read_to_string(path)
            .map_err(|e| Error::Other(format!("read icon source {}: {e}", path.display())))?;
        let document = SvgElement::parse(&source)
            .map_err(|e| Error::Other(format!("{}: {e}", path.display())))?;
        Svgb::new(Svgb::MIFCONV_VERSION)?
            .encode(&document)
            .map_err(|e| Error::Other(format!("{}: {e}", path.display())))
    }

    /// `/OPT file.bmp`: the bitmap at its depth, then for `,8` its `<stem>_mask_soft.bmp`
    /// at `/8` — the two forms `mifconv` was seen to hand `bmconv`.
    fn bitmap_icon(
        source: &IconSource,
        path: &Path,
        name: String,
        depth: &MifDepth,
        bitmaps: &mut Bitmaps,
    ) -> Result<MifIcon> {
        if source.animated {
            return Err(Error::Other(format!(
                "{}: TODO: /A on a bitmap icon (not observed)",
                path.display()
            )));
        }
        let colour = MbmDepth::from_option(&depth.depth)
            .map_err(|e| Error::Other(format!("{}: {e}", path.display())))?;
        let index = bitmaps.push(path, colour)?;
        let mask = match depth.mask {
            None => None,
            Some(8) => {
                let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                let mask = path.with_file_name(format!("{stem}_mask_soft.bmp"));
                if !mask.is_file() {
                    return Err(Error::Other(format!(
                        "{}: mask depth 8 needs {} next to it (mifconv: EGray256 Mask not found)",
                        path.display(),
                        mask.display()
                    )));
                }
                Some(bitmaps.push(&mask, MbmDepth::Grey8)?)
            }
            Some(_) => {
                return Err(Error::Other(format!(
                    "{}: TODO: mask depth 1 on a bitmap icon (not observed: with both mask files \
                     present mifconv took <stem>_mask_soft.bmp at /8, so the /1 form is unknown)",
                    path.display()
                )));
            }
        };
        Ok(MifIcon::bitmap(name, index, mask))
    }

    fn write(path: &Path, bytes: &[u8]) -> Result<()> {
        std::fs::write(path, bytes)
            .map_err(|e| Error::Other(format!("write {}: {e}", path.display())))
    }
}
