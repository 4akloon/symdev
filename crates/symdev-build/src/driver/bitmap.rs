//! `GcceBuild`: compiling one `START BITMAP` block natively (experiment 58).
use std::path::Path;

use symdev_core::{Error, Result};
use symdev_mbm::{BmpImage, MbmBitmap, MbmDepth, MbmFile};

use super::GcceBuild;
use crate::MmpBitmap;
use crate::project::HostPath;

impl GcceBuild {
    /// A `START BITMAP` block into `<name>.mbm` (and `<stem>.mbg` with `HEADER`) by the
    /// native bitmap compiler, byte-equal to the SDK `bmconv` on experiment 58's cases.
    pub(super) fn compile_bitmap(
        &self,
        block: &MmpBitmap,
        mmp_dir: &Path,
        build_dir: &Path,
    ) -> Result<()> {
        if block.sources.is_empty() {
            return Err(Error::Other(format!(
                "START BITMAP {}: no SOURCE line",
                block.target
            )));
        }
        let mut bitmaps = Vec::with_capacity(block.sources.len());
        let mut names = Vec::with_capacity(block.sources.len());
        for source in &block.sources {
            let wanted = source.path(mmp_dir);
            // §7.4 lower-cases every source name; on this host the file may be spelled
            // any way, so the exact name is tried first and then the case is folded.
            let path = if wanted.is_file() {
                wanted
            } else {
                let dir = wanted.parent().unwrap_or(mmp_dir);
                let name = wanted.file_name().and_then(|n| n.to_str()).unwrap_or("");
                HostPath::find(dir, name).ok_or_else(|| {
                    Error::Other(format!("bitmap source not found: {}", wanted.display()))
                })?
            };
            let bytes = std::fs::read(&path)
                .map_err(|e| Error::Other(format!("read {}: {e}", path.display())))?;
            let image = BmpImage::parse(&bytes)
                .map_err(|e| Error::Other(format!("{}: {e}", path.display())))?;
            let depth = MbmDepth::from_option(&source.depth)
                .map_err(|e| Error::Other(format!("{}: {e}", path.display())))?;
            bitmaps.push(MbmBitmap::compile(&image, depth));
            names.push(source.file.clone());
        }
        let mbm = MbmFile::new(bitmaps);
        let out = build_dir.join(block.output()?);
        std::fs::write(&out, mbm.bytes()?)
            .map_err(|e| Error::Other(format!("write {}: {e}", out.display())))?;
        if block.header {
            let header = block.header_name()?;
            let path = build_dir.join(&header);
            std::fs::write(&path, mbm.mbg_text(&header, &names))
                .map_err(|e| Error::Other(format!("write {}: {e}", path.display())))?;
        }
        Ok(())
    }
}
