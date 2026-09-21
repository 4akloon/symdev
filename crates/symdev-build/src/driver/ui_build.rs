//! `RustBuild`: the resource stage a GUI application needs beside the EXE.
//!
//! Three files, none of which a console Rust project gets today: the localisable
//! application resource (the caption and the icon), the registration resource that
//! points at it, and the icon itself. The `.rss` text comes from [`UiResources`]; the
//! compiling is the *existing* native `cpp` + `rcomp` pair, byte-verified on 143 SDK
//! resources (experiment 56), and the *existing* native SVG→MIF encoder. Nothing here
//! writes a binary resource by hand.
use std::path::Path;

use symdev_core::{Artifact, Error, Result};

use super::RustBuild;
use crate::icons::AppIcon;
use crate::ui_resources::UiResources;

impl RustBuild {
    /// Compiles `[ui]`'s two resources and the icon into `build/`, and returns them as
    /// installed artifacts so `SisPackage` picks them up unchanged — which is also
    /// what stops its "no project `_reg.rsc`" fallback from firing and shipping the
    /// captionless registration instead.
    pub(super) fn build_ui(&self, build_dir: &Path) -> Result<Vec<Artifact>> {
        let Some(ui) = &self.ui else {
            return Ok(Vec::new());
        };
        let mut artifacts = Vec::new();
        if let Some(source) = &ui.icon {
            let icon = AppIcon {
                source: source.clone(),
                app: ui.app.clone(),
            };
            self.gcce.compile_icon(&icon, build_dir)?;
            artifacts.push(icon.artifact(build_dir));
        }
        // The application resource first: it is compiled with `HEADER`, and the `.rsg`
        // it writes is what the registration resource includes.
        self.compile_ui_rss(
            ui,
            build_dir,
            &ui.app_rss_path(build_dir),
            ui.app_rss(),
            true,
        )?;
        self.compile_ui_rss(
            ui,
            build_dir,
            &ui.reg_rss_path(build_dir),
            ui.reg_rss(),
            false,
        )?;
        artifacts.extend(ui.artifacts(build_dir));
        Ok(artifacts)
    }

    /// One generated `.rss` through the native preprocessor and resource compiler.
    ///
    /// The include path is the SDK's `epoc32/include` plus `build/`, which is where
    /// the generated sources and the `.rsg` live. There is no project `data/`
    /// directory and no `USERINCLUDE`: everything a generated resource includes is
    /// either the SDK's or symdev's own.
    fn compile_ui_rss(
        &self,
        ui: &UiResources,
        build_dir: &Path,
        rss: &Path,
        text: String,
        header: bool,
    ) -> Result<()> {
        std::fs::write(rss, &text)
            .map_err(|e| Error::Other(format!("write {}: {e}", rss.display())))?;
        let includes = [
            build_dir.to_path_buf(),
            self.gcce.tools.epocroot.join("epoc32/include"),
        ];
        // The C++ path defines `LANGUAGE_<code>` from the `.mmp`'s `LANG`. A Rust
        // project declares no language list, so the resource is built once as the
        // SDK's own default, `SC` — the code `examples/gui` compiles under when its
        // `.mmp` names no `LANG` either.
        let defines = ["LANGUAGE_SC".to_string()];
        let rpp = symdev_rcomp::CPreprocessor::for_rss(&includes, &defines).run(rss)?;
        let compiled = symdev_rcomp::Rcomp::compile(&rpp, &rss.display().to_string())?;
        let rsc = if header {
            ui.app_rsc_path(build_dir)
        } else {
            ui.reg_rsc_path(build_dir)
        };
        std::fs::write(&rsc, compiled.rsc_bytes()?)
            .map_err(|e| Error::Other(format!("write {}: {e}", rsc.display())))?;
        if header {
            let rsg = ui.rsg_path(build_dir);
            std::fs::write(&rsg, compiled.rsg_text())
                .map_err(|e| Error::Other(format!("write {}: {e}", rsg.display())))?;
        }
        Ok(())
    }
}
