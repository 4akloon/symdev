use std::path::{Path, PathBuf};

pub struct UidCrc {
    pub uid1: u32,
    pub uid2: u32,
    pub uid3: u32,
}

impl UidCrc {
    pub fn new(uid1: u32, uid2: u32, uid3: u32) -> Self {
        Self { uid1, uid2, uid3 }
    }

    pub fn checked(&self) -> u32 {
        let mut buf = [0u8; 12];
        buf[0..4].copy_from_slice(&self.uid1.to_le_bytes());
        buf[4..8].copy_from_slice(&self.uid2.to_le_bytes());
        buf[8..12].copy_from_slice(&self.uid3.to_le_bytes());
        let mut even = [0u8; 6];
        let mut odd = [0u8; 6];
        for i in 0..6 {
            even[i] = buf[i * 2];
            odd[i] = buf[i * 2 + 1];
        }
        (u32::from(epoc_crc16(&odd)) << 16) | u32::from(epoc_crc16(&even))
    }

    pub fn bytes(&self) -> [u8; 16] {
        let checked = self.checked();
        let mut out = [0u8; 16];
        out[0..4].copy_from_slice(&self.uid1.to_le_bytes());
        out[4..8].copy_from_slice(&self.uid2.to_le_bytes());
        out[8..12].copy_from_slice(&self.uid3.to_le_bytes());
        out[12..16].copy_from_slice(&checked.to_le_bytes());
        out
    }

    pub fn line(&self) -> String {
        let checked = self.checked();
        format!(
            "0x{:08x} 0x{:08x} 0x{:08x} 0x{:08x}",
            self.uid1, self.uid2, self.uid3, checked
        )
    }
}

pub struct UidCrcTool {
    pub wine: PathBuf,
    pub uidcrc: PathBuf,
}

impl UidCrcTool {
    pub fn new(wine: &Path, uidcrc: &Path) -> Self {
        Self {
            wine: wine.to_path_buf(),
            uidcrc: uidcrc.to_path_buf(),
        }
    }

    pub fn args(&self, crc: &UidCrc, outfile: Option<&str>) -> Vec<String> {
        let mut args = vec![
            self.wine.display().to_string(),
            self.uidcrc.display().to_string(),
            format!("0x{:08x}", crc.uid1),
            format!("0x{:08x}", crc.uid2),
            format!("0x{:08x}", crc.uid3),
        ];
        if let Some(out) = outfile {
            args.push(out.to_string());
        }
        args
    }

    pub fn normalize_stdout(bytes: &[u8]) -> String {
        String::from_utf8_lossy(bytes)
            .trim_matches(['\r', '\n', ' ', '\t'])
            .to_string()
    }

    pub fn output_matches(crc: &UidCrc, stdout: &[u8]) -> bool {
        Self::normalize_stdout(stdout) == crc.line()
    }
}

fn epoc_crc16(data: &[u8]) -> u16 {
    let mut crc: u16 = 0;
    for &b in data {
        crc = crc.rotate_left(8) ^ u16::from(b);
        crc ^= (crc & 0xff) >> 4;
        crc ^= crc << 12;
        crc ^= (crc & 0xff) << 5;
    }
    crc
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn hello() -> UidCrc {
        UidCrc::new(0x1000_007a, 0x1000_39ce, 0xe79e_4cf9)
    }

    fn recorded_tool() -> UidCrcTool {
        UidCrcTool::new(
            Path::new("/usr/bin/wine"),
            Path::new("/sdk/epoc32/tools/uidcrc.exe"),
        )
    }

    #[test]
    fn uid_checked_matches_experiment_13_goldens() {
        let cases = [
            (0x1000_007a, 0x1000_39ce, 0xe79e_4cf9, 0x5dcf_194e),
            (0, 0, 0, 0),
            (0x1000_007a, 0, 0, 0x045a_c39e),
            (0xffff_ffff, 0xffff_ffff, 0xffff_ffff, 0x97df_97df),
            (0x1000_007a, 0x1000_39ce, 0, 0x98a7_d260),
            (0x1234_5678, 0x9abc_def0, 0x1111_1111, 0x3a5f_ebb7),
        ];
        for (u1, u2, u3, checked) in cases {
            assert_eq!(
                UidCrc::new(u1, u2, u3).checked(),
                checked,
                "{u1:#x} {u2:#x} {u3:#x}"
            );
        }
    }

    #[test]
    fn uidcrc_bytes_match_recorded_hello_file() {
        let want = [
            0x7a, 0x00, 0x00, 0x10, 0xce, 0x39, 0x00, 0x10, 0xf9, 0x4c, 0x9e, 0xe7, 0x4e, 0x19,
            0xcf, 0x5d,
        ];
        assert_eq!(hello().bytes(), want);
    }

    #[test]
    fn uidcrc_line_matches_experiment_13_stdout() {
        assert_eq!(
            hello().line(),
            "0x1000007a 0x100039ce 0xe79e4cf9 0x5dcf194e"
        );
    }

    #[test]
    fn uidcrc_args_match_recorded_usage() {
        let tool = recorded_tool();
        let crc = hello();
        assert_eq!(
            tool.args(&crc, None),
            [
                "/usr/bin/wine",
                "/sdk/epoc32/tools/uidcrc.exe",
                "0x1000007a",
                "0x100039ce",
                "0xe79e4cf9",
            ]
        );
        assert_eq!(
            tool.args(&crc, Some("out.uid")),
            [
                "/usr/bin/wine",
                "/sdk/epoc32/tools/uidcrc.exe",
                "0x1000007a",
                "0x100039ce",
                "0xe79e4cf9",
                "out.uid",
            ]
        );
    }

    #[test]
    fn normalize_strips_crlf() {
        assert_eq!(
            UidCrcTool::normalize_stdout(b"0x1000007a 0x100039ce 0xe79e4cf9 0x5dcf194e\r\n"),
            "0x1000007a 0x100039ce 0xe79e4cf9 0x5dcf194e"
        );
    }

    #[test]
    fn uidcrc_output_matches_hello_crlf() {
        let out = b"0x1000007a 0x100039ce 0xe79e4cf9 0x5dcf194e\r\n";
        assert!(UidCrcTool::output_matches(&hello(), out));
    }

    #[test]
    fn uidcrc_output_matches_hello_lf() {
        assert!(UidCrcTool::output_matches(
            &hello(),
            b"0x1000007a 0x100039ce 0xe79e4cf9 0x5dcf194e\n"
        ));
    }

    #[test]
    fn uidcrc_output_rejects_wrong_checked() {
        let out = b"0x1000007a 0x100039ce 0xe79e4cf9 0x00000000\n";
        assert!(!UidCrcTool::output_matches(&hello(), out));
    }
}
