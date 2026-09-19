use std::path::{Path, PathBuf};

use symdev_core::{Error, Result};
use symdev_uidcrc::UidCrc;

mod resource;

pub use resource::{Rsc, RscAppRegistration, RscLtext16, RscResource};

pub struct RscUid {
    pub uid2: u32,
    pub uid3: u32,
}

impl RscUid {
    pub const UID1: u32 = 0x101f_4a6b;
    pub const REGISTRATION_UID2: u32 = 0x101f_8021;

    pub fn new(uid2: u32, uid3: u32) -> Self {
        Self { uid2, uid3 }
    }

    pub fn registration(uid3: u32) -> Self {
        Self::new(Self::REGISTRATION_UID2, uid3)
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

pub struct Rcomp {
    pub unicode: bool,
    pub rsc: String,
    pub header: Option<String>,
    pub source: String,
    pub input: String,
}

impl Rcomp {
    pub fn from_args(args: &[String]) -> Result<Self> {
        let tokens = match args.first().map(String::as_str) {
            Some(s) if !s.starts_with('-') => &args[1..],
            _ => args,
        };
        let mut unicode = false;
        let mut rsc = None;
        let mut header = None;
        let mut source = None;
        let mut input = None;
        for tok in tokens {
            if tok == "-u" {
                unicode = true;
                continue;
            }
            // TODO: rcomp -v -p -l -force -{uid2,uid3} (recorded usage; unused on Wave 0)
            if tok == "-v" || tok == "-p" || tok == "-l" || tok == "-force" || tok.starts_with("-{")
            {
                return Err(Error::Other(format!("TODO: rcomp {tok}")));
            }
            if let Some(v) = tok.strip_prefix("-o") {
                rsc = Some(v.to_string());
            } else if let Some(v) = tok.strip_prefix("-h") {
                header = Some(v.to_string());
            } else if let Some(v) = tok.strip_prefix("-s") {
                source = Some(v.to_string());
            } else if let Some(v) = tok.strip_prefix("-i") {
                input = Some(v.to_string());
            } else if tok.starts_with('-') {
                return Err(Error::Other(format!("unknown rcomp flag: {tok}")));
            } else {
                return Err(Error::Other(format!("unexpected rcomp arg: {tok}")));
            }
        }
        match (rsc, source, input) {
            (Some(rsc), Some(source), Some(input)) => Ok(Self {
                unicode,
                rsc,
                header,
                source,
                input,
            }),
            _ => Err(Error::Other(
                "Usage: rcomp [-u] -oRSCFile [-hHeaderFile] -sSourceFile -iBaseInputFileName"
                    .into(),
            )),
        }
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
        assert_eq!(RscUid::REGISTRATION_UID2, 0x101f_8021);
    }

    #[test]
    fn hello_registration_rsc_matches_experiment_43() {
        let golden = parse_hex(include_str!("testdata/hello_reg.rsc.hex"));
        assert_eq!(golden.len(), 67);
        let rsc = Rsc::registration(0xe79e_4cf9, "hello").unwrap();
        assert_eq!(rsc.bytes().unwrap(), golden);
    }

    #[test]
    fn registration_rsc_rejects_empty_app_file() {
        match Rsc::registration(0xe79e_4cf9, "") {
            Err(err) => assert!(err.to_string().contains("app_file empty")),
            Ok(_) => panic!("expected app_file empty"),
        }
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

    fn driveinfo_reg() -> RscAppRegistration {
        RscAppRegistration::new(
            RscLtext16::new("DriveInfoApp").unwrap(),
            RscLtext16::new("").unwrap(),
            1,
        )
    }

    fn filebrowse_reg() -> RscAppRegistration {
        RscAppRegistration::new(
            RscLtext16::new("filebrowseapp").unwrap(),
            RscLtext16::new("\\resource\\apps\\filebrowseapp_loc").unwrap(),
            1,
        )
    }

    #[test]
    fn driveinfo_rsc_bytes_match_experiment_41() {
        let rsc = Rsc::new(
            RscUid::new(0x101f_8021, 0xa000_01f4),
            vec![driveinfo_reg().resource().unwrap()],
        );
        assert_eq!(rsc.bytes().unwrap(), driveinfo_rsc());
    }

    #[test]
    fn filebrowse_rsc_bytes_match_experiment_41() {
        let rsc = Rsc::new(
            RscUid::new(0x101f_8021, 0xe800_00a6),
            vec![filebrowse_reg().resource().unwrap()],
        );
        assert_eq!(rsc.bytes().unwrap(), filebrowse_rsc());
    }

    #[test]
    fn rsc_ltext16_empty_is_a_zero_length_byte() {
        assert_eq!(RscLtext16::empty().bytes(), [0]);
    }

    #[test]
    fn rsc_ltext16_driveinfo_is_length_pad_and_utf16le() {
        let bytes = RscLtext16::new("DriveInfoApp").unwrap().bytes();
        assert_eq!(bytes[0], 12);
        assert_eq!(bytes[1], 0);
        assert_eq!(
            &bytes[2..],
            "DriveInfoApp"
                .encode_utf16()
                .flat_map(u16::to_le_bytes)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn driveinfo_uncompressed_is_59_bytes_experiment_42() {
        let resource = driveinfo_reg().resource().unwrap();
        assert_eq!(resource.uncompressed().len(), 59);
        assert_eq!(resource.uncompressed_len().unwrap(), 59);
        assert_eq!(&driveinfo_rsc()[16..20], [0, 59, 0, 1]);
        assert_eq!(&driveinfo_rsc()[70..], [0x14, 0, 0x46, 0]);
    }

    #[test]
    fn filebrowse_uncompressed_is_126_bytes_experiment_42() {
        let resource = filebrowse_reg().resource().unwrap();
        assert_eq!(resource.uncompressed().len(), 126);
        assert_eq!(&filebrowse_rsc()[16..20], [0, 126, 0, 1]);
        assert_eq!(&filebrowse_rsc()[105..], [0x14, 0, 0x69, 0]);
    }

    #[test]
    fn rsc_ltext16_rejects_more_than_255_utf16_units() {
        match RscLtext16::new("a".repeat(256)) {
            Err(err) => assert!(err.to_string().contains("LText16 longer than 255")),
            Ok(_) => panic!("expected LText16 longer than 255"),
        }
    }
}

#[cfg(test)]
mod native_tests {
    use super::*;

    #[test]
    fn from_args_match_experiment_9() {
        let r = Rcomp::from_args(&[
            "rcomp".into(),
            "-u".into(),
            "-odriveinfo_reg.rsc".into(),
            "-sdriveinfo_reg.rpp".into(),
            "-idriveinfo_reg.rss".into(),
        ])
        .unwrap();
        assert!(r.unicode);
        assert_eq!(r.rsc, "driveinfo_reg.rsc");
        assert_eq!(r.header, None);
        assert_eq!(r.source, "driveinfo_reg.rpp");
        assert_eq!(r.input, "driveinfo_reg.rss");
    }

    #[test]
    fn from_args_with_header_match_experiment_41() {
        let r = Rcomp::from_args(&[
            "rcomp".into(),
            "-u".into(),
            "-odriveinfo_reg.rsc".into(),
            "-hdriveinfo_reg.rsg".into(),
            "-sdriveinfo_reg.rpp".into(),
            "-idriveinfo_reg.rss".into(),
        ])
        .unwrap();
        assert_eq!(r.header.as_deref(), Some("driveinfo_reg.rsg"));
    }
}
