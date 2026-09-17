use std::path::Path;

pub fn uid_checked(uid1: u32, uid2: u32, uid3: u32) -> u32 {
    let mut buf = [0u8; 12];
    buf[0..4].copy_from_slice(&uid1.to_le_bytes());
    buf[4..8].copy_from_slice(&uid2.to_le_bytes());
    buf[8..12].copy_from_slice(&uid3.to_le_bytes());
    let mut even = [0u8; 6];
    let mut odd = [0u8; 6];
    for i in 0..6 {
        even[i] = buf[i * 2];
        odd[i] = buf[i * 2 + 1];
    }
    (u32::from(epoc_crc16(&odd)) << 16) | u32::from(epoc_crc16(&even))
}

pub fn uidcrc_bytes(uid1: u32, uid2: u32, uid3: u32) -> [u8; 16] {
    let checked = uid_checked(uid1, uid2, uid3);
    let mut out = [0u8; 16];
    out[0..4].copy_from_slice(&uid1.to_le_bytes());
    out[4..8].copy_from_slice(&uid2.to_le_bytes());
    out[8..12].copy_from_slice(&uid3.to_le_bytes());
    out[12..16].copy_from_slice(&checked.to_le_bytes());
    out
}

pub fn uidcrc_line(uid1: u32, uid2: u32, uid3: u32) -> String {
    let checked = uid_checked(uid1, uid2, uid3);
    format!("0x{uid1:08x} 0x{uid2:08x} 0x{uid3:08x} 0x{checked:08x}")
}

pub fn uidcrc_args(
    wine: &Path,
    uidcrc: &Path,
    uid1: u32,
    uid2: u32,
    uid3: u32,
    outfile: Option<&str>,
) -> Vec<String> {
    let mut args = vec![
        wine.display().to_string(),
        uidcrc.display().to_string(),
        format!("0x{uid1:08x}"),
        format!("0x{uid2:08x}"),
        format!("0x{uid3:08x}"),
    ];
    if let Some(out) = outfile {
        args.push(out.to_string());
    }
    args
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
            assert_eq!(uid_checked(u1, u2, u3), checked, "{u1:#x} {u2:#x} {u3:#x}");
        }
    }

    #[test]
    fn uidcrc_bytes_match_recorded_hello_file() {
        let got = uidcrc_bytes(0x1000_007a, 0x1000_39ce, 0xe79e_4cf9);
        let want = [
            0x7a, 0x00, 0x00, 0x10, 0xce, 0x39, 0x00, 0x10, 0xf9, 0x4c, 0x9e, 0xe7, 0x4e, 0x19,
            0xcf, 0x5d,
        ];
        assert_eq!(got, want);
    }

    #[test]
    fn uidcrc_line_matches_experiment_13_stdout() {
        assert_eq!(
            uidcrc_line(0x1000_007a, 0x1000_39ce, 0xe79e_4cf9),
            "0x1000007a 0x100039ce 0xe79e4cf9 0x5dcf194e"
        );
    }

    #[test]
    fn uidcrc_args_match_recorded_usage() {
        let wine = Path::new("/usr/bin/wine");
        let exe = Path::new("/sdk/epoc32/tools/uidcrc.exe");
        assert_eq!(
            uidcrc_args(wine, exe, 0x1000_007a, 0x1000_39ce, 0xe79e_4cf9, None),
            [
                "/usr/bin/wine",
                "/sdk/epoc32/tools/uidcrc.exe",
                "0x1000007a",
                "0x100039ce",
                "0xe79e4cf9",
            ]
        );
        assert_eq!(
            uidcrc_args(
                wine,
                exe,
                0x1000_007a,
                0x1000_39ce,
                0xe79e_4cf9,
                Some("out.uid")
            ),
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
}
