use std::path::{Path, PathBuf};

use crate::uidcrc::UidCrc;

pub struct RscUid {
    pub uid2: u32,
    pub uid3: u32,
}

impl RscUid {
    pub const UID1: u32 = 0x101f_4a6b;

    pub fn new(uid2: u32, uid3: u32) -> Self {
        Self { uid2, uid3 }
    }

    pub fn crc(&self) -> UidCrc {
        UidCrc::new(Self::UID1, self.uid2, self.uid3)
    }

    pub fn bytes(&self) -> [u8; 16] {
        self.crc().bytes()
    }
}

pub struct RcompTool {
    pub wine: PathBuf,
    pub rcomp: PathBuf,
}

impl RcompTool {
    pub fn new(wine: &Path, rcomp: &Path) -> Self {
        Self {
            wine: wine.to_path_buf(),
            rcomp: rcomp.to_path_buf(),
        }
    }

    pub fn args(&self, rsc: &str, source: &str, input: &str) -> Vec<String> {
        vec![
            self.wine.display().to_string(),
            self.rcomp.display().to_string(),
            "-u".into(),
            format!("-o{rsc}"),
            format!("-s{source}"),
            format!("-i{input}"),
        ]
    }

    pub fn args_with_header(
        &self,
        rsc: &str,
        header: &str,
        source: &str,
        input: &str,
    ) -> Vec<String> {
        vec![
            self.wine.display().to_string(),
            self.rcomp.display().to_string(),
            "-u".into(),
            format!("-o{rsc}"),
            format!("-h{header}"),
            format!("-s{source}"),
            format!("-i{input}"),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn parse_hex(s: &str) -> Vec<u8> {
        let hex: String = s.chars().filter(|c| !c.is_whitespace()).collect();
        (0..hex.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
            .collect()
    }

    fn driveinfo_rsc() -> Vec<u8> {
        parse_hex(include_str!("testdata/driveinfo_reg.rsc.hex"))
    }

    fn filebrowse_rsc() -> Vec<u8> {
        parse_hex(include_str!("testdata/filebrowseapp_reg.rsc.hex"))
    }

    fn recorded_tool() -> RcompTool {
        RcompTool::new(
            Path::new("/usr/bin/wine"),
            Path::new("/sdk/epoc32/tools/rcomp.exe"),
        )
    }

    #[test]
    fn driveinfo_rsc_uid_bytes_match_experiment_41() {
        let golden = driveinfo_rsc();
        assert_eq!(golden.len(), 74);
        let want = [
            0x6b, 0x4a, 0x1f, 0x10, 0x21, 0x80, 0x1f, 0x10, 0xf4, 0x01, 0x00, 0xa0, 0xb4, 0x0c,
            0xc8, 0xf0,
        ];
        assert_eq!(RscUid::new(0x101f_8021, 0xa000_01f4).bytes(), want);
        assert_eq!(&golden[..16], &want);
    }

    #[test]
    fn driveinfo_rsc_uid_checked_matches_experiment_41() {
        assert_eq!(
            RscUid::new(0x101f_8021, 0xa000_01f4).crc().checked(),
            0xf0c8_0cb4
        );
    }

    #[test]
    fn filebrowse_rsc_uid_bytes_match_experiment_41() {
        let golden = filebrowse_rsc();
        assert_eq!(golden.len(), 109);
        let want = [
            0x6b, 0x4a, 0x1f, 0x10, 0x21, 0x80, 0x1f, 0x10, 0xa6, 0x00, 0x00, 0xe8, 0x69, 0x64,
            0x35, 0x0a,
        ];
        assert_eq!(RscUid::new(0x101f_8021, 0xe800_00a6).bytes(), want);
        assert_eq!(&golden[..16], &want);
    }

    #[test]
    fn rsc_uid1_is_recorded_unicode_resource_file() {
        assert_eq!(RscUid::UID1, 0x101f_4a6b);
    }

    #[test]
    fn rcomp_args_match_experiment_9() {
        assert_eq!(
            recorded_tool().args(
                "driveinfo_reg.rsc",
                "driveinfo_reg.rpp",
                "driveinfo_reg.rss"
            ),
            [
                "/usr/bin/wine",
                "/sdk/epoc32/tools/rcomp.exe",
                "-u",
                "-odriveinfo_reg.rsc",
                "-sdriveinfo_reg.rpp",
                "-idriveinfo_reg.rss",
            ]
        );
    }

    #[test]
    fn rcomp_args_with_header_match_experiment_41() {
        assert_eq!(
            recorded_tool().args_with_header(
                "driveinfo_reg.rsc",
                "driveinfo_reg.rsg",
                "driveinfo_reg.rpp",
                "driveinfo_reg.rss"
            ),
            [
                "/usr/bin/wine",
                "/sdk/epoc32/tools/rcomp.exe",
                "-u",
                "-odriveinfo_reg.rsc",
                "-hdriveinfo_reg.rsg",
                "-sdriveinfo_reg.rpp",
                "-idriveinfo_reg.rss",
            ]
        );
    }
}
