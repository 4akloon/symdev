//! `GcceBuild`: compiling one `.rss` resource natively (experiment 56).
use std::path::{Path, PathBuf};

use symdev_core::{Error, Result};

use super::GcceBuild;
use crate::resources::MmpPath;
use crate::{Mmp, MmpResource};

impl GcceBuild {
    /// `.rss` → `.rsc` (and `.rsg` with `HEADER`) by the native preprocessor and
    /// compiler (experiment 56: byte-equal to the SDK `cpp.exe` + `rcomp.exe` on the SDK
    /// examples).
    pub(super) fn compile_resource(
        &self,
        res: &MmpResource,
        mmp_dir: &Path,
        mmp: &Mmp,
        build_dir: &Path,
    ) -> Result<()> {
        let rss = res.source(mmp_dir);
        if !rss.is_file() {
            return Err(Error::Other(format!(
                "resource not found: {}",
                rss.display()
            )));
        }
        let epoc = self.tools.epocroot.join("epoc32");
        let mut includes = vec![
            rss.parent().unwrap_or(mmp_dir).to_path_buf(),
            build_dir.to_path_buf(),
        ];
        includes.extend(
            mmp.userinclude
                .iter()
                .map(|d| self.mmp_dir_path(mmp_dir, d)),
        );
        includes.push(epoc.join("include"));
        includes.extend(
            mmp.systeminclude
                .iter()
                .map(|d| self.mmp_dir_path(mmp_dir, d)),
        );
        // §6.3: one compiled resource per language, one `.rsg` whatever the list is.
        for language in res.languages(&mmp.lang) {
            // §6.4: the `.mmp`'s own `MACRO` list and the language code, not the
            // platform macros.
            let mut defines = mmp.macros.clone();
            defines.push(format!("LANGUAGE_{language}"));
            let rpp = symdev_rcomp::CPreprocessor::for_rss(&includes, &defines).run(&rss)?;
            let compiled = symdev_rcomp::Rcomp::compile(&rpp, &rss.display().to_string())?;
            if !res.headeronly {
                let rsc = build_dir.join(res.output(&language)?);
                std::fs::write(&rsc, compiled.rsc_bytes()?)
                    .map_err(|e| Error::Other(format!("write {}: {e}", rsc.display())))?;
            }
            if res.header || res.headeronly {
                let rsg = build_dir.join(res.header_name()?);
                std::fs::write(&rsg, compiled.rsg_text())
                    .map_err(|e| Error::Other(format!("write {}: {e}", rsg.display())))?;
            }
        }
        Ok(())
    }

    /// An MMP include directory, resolved against the MMP or the SDK root (`MmpPath`).
    pub(super) fn mmp_dir_path(&self, mmp_dir: &Path, dir: &str) -> PathBuf {
        MmpPath::resolve(mmp_dir, &self.tools.epocroot, dir)
    }
}
