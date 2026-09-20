//! `GcceBuild`: compiling the `[symbian] icon` SVG to a `.mif`.
use std::path::Path;

use symdev_core::{Error, RemotePath, Result};

use super::{GcceBuild, arg, io};
use crate::icons::{AppIcon, MifConvTool, MifFile};

impl GcceBuild {
    /// `.rss` → `.rsc` (+ `.rsg` with `HEADER`) with the SDK `cpp.exe` and `rcomp.exe`
    /// under Wine (experiment 9 argv, Wine `Z:` paths).
    /// `<app>_aif.mif` from the SVG via Wine `mifconv` (experiment 55). It runs in
    /// `build/mifconv-temp` on a copy named `<app>.svg` with relative paths: the input
    /// path becomes part of its temporary `.svgb` name, and with a long one
    /// `svgtbinencode` silently writes nothing (a MIF with empty icons).
    pub(super) fn compile_icon(&self, icon: &AppIcon, build_dir: &Path) -> Result<()> {
        if !icon.source.is_file() {
            return Err(Error::Other(format!(
                "icon not found: {}",
                icon.source.display()
            )));
        }
        let tools = self.tools.epocroot.join("epoc32/tools");
        let work = build_dir.join("mifconv-temp");
        if work.exists() {
            std::fs::remove_dir_all(&work).map_err(io)?;
        }
        std::fs::create_dir_all(work.join("t")).map_err(io)?;
        let svg = format!("{}.svg", icon.app);
        std::fs::copy(&icon.source, work.join(&svg)).map_err(io)?;
        let mif = format!("{}_aif.mif", icon.app);
        let mbg = format!("{}_aif.mbg", icon.app);
        let tool = MifConvTool {
            wine: self.tools.wine.clone(),
            mifconv: tools.join("mifconv.exe"),
            encoder_dir: tools,
        };
        self.run_tool_env(
            &tool.args(&mif, &mbg, "t", &svg),
            &RemotePath::new(arg(&work)),
            &[("WINEDEBUG", "-all".into())],
        )?;
        // mifconv exits 0 after printing its usage on bad input: check what it wrote.
        let bytes = std::fs::read(work.join(&mif))
            .map_err(|e| Error::Other(format!("mifconv wrote no {mif}: {e}")))?;
        MifFile::check(&bytes).map_err(|e| Error::Other(format!("{mif}: {e}")))?;
        std::fs::rename(work.join(&mif), icon.mif(build_dir)).map_err(io)?;
        std::fs::rename(work.join(&mbg), icon.mbg(build_dir)).map_err(io)?;
        Ok(())
    }
}
