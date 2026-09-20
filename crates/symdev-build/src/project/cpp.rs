//! `ProjectCpp`: the C preprocessor pass the SDK runs over `bld.inf` and `.mmp`.
use std::path::{Path, PathBuf};

use symdev_core::{Error, Result};
use symdev_rcomp::CPreprocessor;

use super::HostPath;

/// The six macros a GCCE build defines for a project file (mmp-frontend-spec.md §1.3).
/// Longest first: `restore` undoes the expansion by plain text replacement.
const GCCE_MACROS: [&str; 6] = [
    "GENERIC_MARM",
    "MARM_ARMV5",
    "EPOC32",
    "GCCE",
    "MARM",
    "EABI",
];

/// Which preprocessing run this is (§1.6). `bld.inf` is preprocessed twice: once with no
/// macros at all — so a `#ifdef GCCE` around `PRJ_EXPORTS` is always false — and once per
/// platform. A `.mmp` is preprocessed once, per platform.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectPass {
    /// `bld.inf` first pass: the variant header, no `-D` at all.
    Platform,
    /// `.mmp`, and `bld.inf`'s second pass: the GCCE macro set.
    Gcce,
}

/// Preprocessing of one project file, against one SDK.
///
/// Faithful to the SDK in the part that surprises people: every platform macro expands
/// to its own name with five leading underscores, which is undone before tokenising
/// (§1.4). `#ifdef GCCE` is therefore true while `#if GCCE` is false (§14.3), exactly as
/// the SDK behaves, and `OPTION GCCE -O3` survives as written.
pub struct ProjectCpp {
    epocroot: PathBuf,
}

impl ProjectCpp {
    pub fn new(epocroot: &Path) -> Self {
        Self {
            epocroot: epocroot.to_path_buf(),
        }
    }

    /// The preprocessed text of one `bld.inf` or `.mmp`.
    pub fn run(&self, file: &Path, pass: ProjectPass) -> Result<String> {
        let variant = self.variant_header()?;
        let dirs = vec![
            self.epocroot.join("epoc32/include"),
            file.parent().unwrap_or(Path::new(".")).to_path_buf(),
            variant.parent().unwrap_or(Path::new(".")).to_path_buf(),
        ];
        let defines: Vec<String> = match pass {
            ProjectPass::Platform => Vec::new(),
            ProjectPass::Gcce => GCCE_MACROS
                .iter()
                .map(|name| format!("{name}=_____{name}"))
                .collect(),
        };
        let bytes = CPreprocessor::new(&dirs, &defines)
            .preinclude(&variant)
            .run(file)
            .map_err(|e| Self::explain(file, &e))?;
        let text: String = bytes.iter().map(|&b| char::from(b)).collect();
        Ok(Self::restore(&text))
    }

    /// The `.hrh` `epoc32/tools/variant/variant.cfg` names, force-included into every
    /// project file (§1.5). Its path is spelled in the SDK's own case, which is not the
    /// case on disk.
    pub fn variant_header(&self) -> Result<PathBuf> {
        let cfg = self.epocroot.join("epoc32/tools/variant/variant.cfg");
        let text = std::fs::read_to_string(&cfg)
            .map_err(|e| Error::Other(format!("read {}: {e}", cfg.display())))?;
        let named = text
            .lines()
            .map(str::trim)
            .find(|line| !line.starts_with('#') && line.to_ascii_lowercase().ends_with(".hrh"))
            .ok_or_else(|| Error::Other(format!("{}: no variant .hrh line", cfg.display())))?;
        HostPath::find(&self.epocroot, named).ok_or_else(|| {
            Error::Other(format!(
                "{}: the variant header {named} is not in {}",
                cfg.display(),
                self.epocroot.display()
            ))
        })
    }

    /// Undo the `_____NAME` expansion of §1.4.
    ///
    /// Only the text is restored, not the spacing: symdev's preprocessor pads every
    /// expansion with a space on both sides where the SDK's GCC 2.x one adds one after.
    /// That is invisible to the tokeniser except for a platform macro spelled inside a
    /// larger word — `..\GCCE\src` splits here into three tokens and into two in the
    /// SDK. Both are broken; neither is worth reproducing.
    fn restore(text: &str) -> String {
        let mut out = text.to_string();
        for name in GCCE_MACROS {
            out = out.replace(&format!("_____{name}"), name);
        }
        out
    }

    /// §13: an `.mmp` copied out of a Symbian platform source tree fails on an include
    /// this SDK does not have. Say which, and what to write instead.
    fn explain(file: &Path, err: &Error) -> Error {
        let message = err.to_string();
        let advice = if message.to_ascii_lowercase().contains("platform_paths.hrh") {
            "\nplatform_paths.hrh is part of a Symbian platform source tree, not of this SDK. \
             Replace its layer macros (MW_LAYER_SYSTEMINCLUDE and friends) with explicit \
             SYSTEMINCLUDE lines; for an ordinary S60 3rd FP2 application \
             `SYSTEMINCLUDE \\epoc32\\include` and `SYSTEMINCLUDE \\epoc32\\include\\variant` \
             cover it."
        } else {
            ""
        };
        Error::Other(format!("{}: {message}{advice}", file.display()))
    }
}
