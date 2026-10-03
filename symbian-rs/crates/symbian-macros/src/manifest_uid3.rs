//! `ManifestUid3`: `[symbian] uid3` of the application's `symdev.toml`, read while the
//! application compiles. A proc macro runs in the rustc that compiles the application, so
//! its `CARGO_MANIFEST_DIR` is the application's; a dependency's build script would see
//! its own (experiment 114 §1, design spec §3).
use std::path::Path;

pub struct ManifestUid3;

impl ManifestUid3 {
    pub fn read(dir: &Path) -> Result<u32, String> {
        let path = dir.join("symdev.toml");
        let text = std::fs::read_to_string(&path)
            .map_err(|e| format!("`symbian_std::uid3!()` reads {}: {e}", path.display()))?;
        Self::parse(&text).map_err(|e| format!("{}: {e}", path.display()))
    }

    /// The scaffold's shape: a `[symbian]` table with `uid3 = "0x<1–8 hex digits>"`.
    pub fn parse(text: &str) -> Result<u32, String> {
        let mut in_symbian = false;
        for line in text.lines() {
            let line = line.split('#').next().unwrap_or("").trim();
            if line.starts_with('[') {
                in_symbian = line == "[symbian]";
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            if !in_symbian || key.trim() != "uid3" {
                continue;
            }
            let hex = value
                .trim()
                .strip_prefix("\"0x")
                .or_else(|| value.trim().strip_prefix("\"0X"))
                .and_then(|v| v.strip_suffix('"'))
                .filter(|h| (1..=8).contains(&h.len()))
                .ok_or_else(|| {
                    format!(
                        "[symbian] uid3 must be a quoted hex number like \"0xe0000687\", not {}",
                        value.trim()
                    )
                })?;
            return u32::from_str_radix(hex, 16).map_err(|e| format!("[symbian] uid3 {hex}: {e}"));
        }
        Err("no [symbian] uid3 (symdev new writes one)".into())
    }
}

#[cfg(test)]
mod tests {
    use super::ManifestUid3;

    const SCAFFOLD: &str = "[package]\nname = \"hello\"\nversion = \"0.1.0\"\n\n[target]\n\
        device = \"nokia-e52\"\n\n[language]\nname = \"rust\"\n\n[symbian]\nuid3 = \"0xef9f2cab\"\n\
        capabilities = []\nvendor = \"symdev\"\n\n[signing]\nmode = \"self-signed\"\n";

    #[test]
    fn the_scaffolds_uid3_is_read() {
        assert_eq!(ManifestUid3::parse(SCAFFOLD), Ok(0xef9f_2cab));
    }

    #[test]
    fn a_uid3_outside_the_symbian_table_does_not_count() {
        let text = "[package]\nuid3 = \"0x1\"\n[symbian]\nvendor = \"x\"\n";
        assert!(
            ManifestUid3::parse(text)
                .unwrap_err()
                .contains("[symbian] uid3")
        );
    }

    #[test]
    fn a_uid3_that_is_not_quoted_hex_is_refused() {
        for bad in [
            "uid3 = 0xe1",
            "uid3 = \"e1\"",
            "uid3 = \"0xZZ\"",
            "uid3 = \"0x123456789\"",
        ] {
            let text = format!("[symbian]\n{bad}\n");
            assert!(ManifestUid3::parse(&text).is_err(), "{bad}");
        }
    }

    #[test]
    fn spaces_and_comments_around_it_are_fine() {
        assert_eq!(
            ManifestUid3::parse("[symbian]\n  uid3=\"0xE0000687\"  # app\n"),
            Ok(0xe000_0687)
        );
    }
}
