//! `SisPkgFile`: a non-EXE file to install, and its E32 capability sniffing.
use crate::SisWord41;

/// A non-EXE file to install: its `!:\...` destination and bytes.
#[derive(Debug, Clone, Copy)]
pub struct SisPkgFile<'a> {
    pub dest: &'a str,
    pub data: &'a [u8],
}

impl<'a> SisPkgFile<'a> {
    /// makesis takes type 41 from an E32 file's own header (`EPOC` at 16, `iCaps` at
    /// 0x88): observed for a DLL next to its EXE (experiment 53).
    pub(super) fn e32_capabilities(&self) -> Option<SisWord41> {
        if self.data.get(16..20) != Some(b"EPOC".as_slice()) {
            return None;
        }
        let caps = self.data.get(0x88..0x8c)?;
        Some(SisWord41::new(u32::from_le_bytes([
            caps[0], caps[1], caps[2], caps[3],
        ])))
    }

    /// `!:\private\10003a3f\import\apps\<name>_reg.rsc` (experiment 43 template).
    pub fn reg_rsc_dest(name: &str) -> String {
        format!("!:\\private\\10003a3f\\import\\apps\\{name}_reg.rsc")
    }
}
