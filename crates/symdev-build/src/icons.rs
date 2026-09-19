use std::path::{Path, PathBuf};

use symdev_core::{Artifact, Error, Project, Result};

use crate::resources::ProjectMmps;

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
        let app = ProjectMmps::load(project)?
            .mmps
            .into_iter()
            .map(|(_, mmp)| mmp)
            .find(|mmp| mmp.target_type.eq_ignore_ascii_case("EXE"))
            .ok_or_else(|| Error::Other("icon set but the project has no EXE MMP".into()))?
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

/// A multi-icon file as `mifconv` writes it (experiment 55): `B##4`, version, offset of
/// the entry table, entry count; entries `(offset, length)`; each points at a `C##4`
/// icon header whose data length is at +12.
pub struct MifFile;

impl MifFile {
    /// Every icon entry has data (an empty `.svgb` from `svgtbinencode` gives length 0).
    pub fn check(bytes: &[u8]) -> Result<()> {
        let u32_at = |at: usize| {
            bytes
                .get(at..at + 4)
                .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
                .ok_or_else(|| Error::Other(format!("MIF truncated at {at:#x}")))
        };
        if bytes.get(..4) != Some(b"B##4".as_slice()) {
            return Err(Error::Other("not a MIF (no B##4)".into()));
        }
        let table = u32_at(8)? as usize;
        let count = u32_at(12)? as usize;
        if count == 0 {
            return Err(Error::Other("MIF has no icons".into()));
        }
        for i in 0..count {
            let at = u32_at(table + 8 * i)? as usize;
            if bytes.get(at..at + 4) != Some(b"C##4".as_slice()) {
                return Err(Error::Other(format!("MIF entry {i}: no C##4 at {at:#x}")));
            }
            if u32_at(at + 12)? == 0 {
                return Err(Error::Other(format!(
                    "MIF entry {i} is empty (svgtbinencode produced nothing)"
                )));
            }
        }
        Ok(())
    }
}

/// SDK `mifconv.exe` under Wine (experiment 55). It calls `svgtbinencode.exe` from the
/// `/S` directory and needs a writable `/T` temp directory (its default
/// `\epoc32\BUILD\s60\icons\temp\` does not exist here).
pub struct MifConvTool {
    pub wine: PathBuf,
    pub mifconv: PathBuf,
    /// Directory holding `svgtbinencode.exe` (`epoc32/tools`).
    pub encoder_dir: PathBuf,
}

impl MifConvTool {
    /// `/c32,8`: 32-bit colour with an 8-bit mask, as the SDK example icon makefiles.
    pub const DEPTH: &str = "/c32,8";

    /// `/a/b` → `Z:\\a\\b` (Wine maps `/` to drive `Z:`).
    fn wine_path(path: &Path) -> String {
        format!("Z:{}", path.display().to_string().replace('/', "\\"))
    }

    pub fn args(&self, mif: &str, mbg: &str, temp: &str, svg: &str) -> Vec<String> {
        vec![
            self.wine.display().to_string(),
            self.mifconv.display().to_string(),
            mif.to_string(),
            format!("/H{mbg}"),
            format!("/S{}", Self::wine_path(&self.encoder_dir)),
            format!("/T{temp}"),
            Self::DEPTH.to_string(),
            svg.to_string(),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mifconv_argv_matches_experiment_55() {
        let tool = MifConvTool {
            wine: PathBuf::from("/usr/bin/wine"),
            mifconv: PathBuf::from("/sdk/epoc32/tools/mifconv.exe"),
            encoder_dir: PathBuf::from("/sdk/epoc32/tools"),
        };
        assert_eq!(
            tool.args(
                "Z:\\p\\a_aif.mif",
                "Z:\\p\\a_aif.mbg",
                "Z:\\p\\t",
                "Z:\\p\\a.svg"
            ),
            [
                "/usr/bin/wine",
                "/sdk/epoc32/tools/mifconv.exe",
                "Z:\\p\\a_aif.mif",
                "/HZ:\\p\\a_aif.mbg",
                "/SZ:\\sdk\\epoc32\\tools",
                "/TZ:\\p\\t",
                "/c32,8",
                "Z:\\p\\a.svg",
            ]
        );
    }

    fn hex(s: &str) -> Vec<u8> {
        let s: String = s.split_whitespace().collect();
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
            .collect()
    }

    #[test]
    fn mif_check_rejects_the_empty_icon_mifconv_wrote_for_a_long_path() {
        // Experiment 55: the 64-byte MIF with zero-length icon data.
        let empty = hex(
            "42232334 02000000 10000000 02000000 20000000 20000000 20000000 20000000
             43232334 01000000 20000000 00000000 01000000 0b000000 00000000 04000000",
        );
        let err = MifFile::check(&empty).unwrap_err().to_string();
        assert!(err.contains("empty"), "{err}");
        let mut good = empty.clone();
        good[0x2c] = 4;
        good.extend_from_slice(&[1, 2, 3, 4]);
        MifFile::check(&good).unwrap();
    }

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
