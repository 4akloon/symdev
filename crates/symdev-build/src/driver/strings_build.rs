//! `RustBuild`: the per-language strings files, compiled beside the EXE.
//!
//! The text comes from [`StringsResources`]; the compiling is the same native `cpp` +
//! `rcomp` pair the application resources go through, byte-verified on 143 SDK
//! resources (experiment 56). Nothing here encodes a resource by hand.
use std::path::Path;

use symdev_core::{Artifact, Error, Result};
use symdev_locale::Locales;

use super::RustBuild;
use crate::strings_resources::StringsResources;

impl RustBuild {
    /// Compiles `locales/` into `build/<app>_strings.rsc` and one `.r<code>` per
    /// language, and returns them as installed artifacts. A project with no `locales/`
    /// gets nothing and pays nothing.
    pub(super) fn build_strings(&self, root: &Path, build_dir: &Path) -> Result<Vec<Artifact>> {
        let dir = root.join("locales");
        let Some(locales) = Locales::load(&dir).map_err(|e| Error::Other(e.to_string()))? else {
            return Ok(Vec::new());
        };
        let strings = StringsResources {
            app: self.name.clone(),
            locales,
        };
        if !strings.has_strings() {
            return Ok(Vec::new());
        }
        for (language, table) in strings.languages() {
            let rss = strings.rss_path(build_dir, language);
            std::fs::write(&rss, strings.rss(table))
                .map_err(|e| Error::Other(format!("write {}: {e}", rss.display())))?;
            // No include path and no defines: the generated source includes nothing.
            let rpp = symdev_rcomp::CPreprocessor::for_rss(&[], &[]).run(&rss)?;
            let compiled = symdev_rcomp::Rcomp::compile(&rpp, &rss.display().to_string())?;
            let rsc = strings.rsc_path(build_dir, language);
            std::fs::write(&rsc, compiled.rsc_bytes()?)
                .map_err(|e| Error::Other(format!("write {}: {e}", rsc.display())))?;
        }
        Ok(strings.artifacts(build_dir))
    }
}
