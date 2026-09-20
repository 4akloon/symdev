//! `Wave0Pkg`: the Wave-0 subset of `.pkg` text makesis parses.
use std::path::PathBuf;

use symdev_core::{Error, Result};

pub(super) struct Wave0Pkg {
    pub(super) name: String,
    pub(super) uid3: u32,
    pub(super) version: (u32, u32, u32),
    pub(super) vendor: String,
    pub(super) vendor_localized: String,
    pub(super) exe: PathBuf,
    /// Non-EXE `(source, destination)` lines in `.pkg` order.
    pub(super) files: Vec<(PathBuf, String)>,
}

impl Wave0Pkg {
    pub(super) fn parse(text: &str) -> Result<Self> {
        let mut name = None;
        let mut uid3 = None;
        let mut version = None;
        let mut vendor_localized = None;
        let mut vendor = None;
        let mut exe = None;
        let mut files = Vec::new();
        let mut saw_en = false;
        let mut saw_platform = false;
        for raw in text.lines() {
            let line = raw.trim();
            // `;` comment lines as in the SDK example pkg (experiment 7).
            if line.is_empty() || line.starts_with(';') {
                continue;
            }
            if line == "&EN" {
                saw_en = true;
                continue;
            }
            if let Some(rest) = line.strip_prefix("#{") {
                let (n, rest) = Self::quoted(rest)?;
                let rest = rest
                    .strip_prefix("},(")
                    .ok_or_else(|| Error::Other("pkg header missing UID".into()))?;
                let (uid_tok, rest) = rest
                    .split_once("),")
                    .ok_or_else(|| Error::Other("pkg header missing version".into()))?;
                let uid = Self::parse_uid3(uid_tok)?;
                let (maj, rest) = Self::split_u32(rest)?;
                let (min, rest) = Self::split_u32(rest)?;
                let (build, rest) = Self::split_u32(rest)?;
                if rest != "TYPE=SA" {
                    return Err(Error::Other(format!(
                        "TODO: pkg type other than TYPE=SA: {rest}"
                    )));
                }
                name = Some(n);
                uid3 = Some(uid);
                version = Some((maj, min, build));
                continue;
            }
            if let Some(rest) = line.strip_prefix("%{") {
                let (v, rest) = Self::quoted(rest)?;
                if rest != "}" {
                    return Err(Error::Other("pkg localized vendor line".into()));
                }
                vendor_localized = Some(v);
                continue;
            }
            if let Some(rest) = line.strip_prefix(':') {
                vendor = Some(Self::unquote(rest)?);
                continue;
            }
            if line.starts_with("[0x102752AE]") || line.starts_with("[0x102752ae]") {
                saw_platform = true;
                continue;
            }
            if line.starts_with('[') {
                return Err(Error::Other(format!(
                    "TODO: pkg platform UID other than 0x102752AE: {line}"
                )));
            }
            if line.starts_with('"') {
                let (src, rest) = Self::quoted(line)?;
                let dest = rest
                    .trim_start()
                    .strip_prefix('-')
                    .ok_or_else(|| Error::Other(format!("pkg file line missing dest: {line}")))?;
                let dest = Self::unquote(dest)?;
                if Self::is_exe_dest(&dest) {
                    if exe.replace((PathBuf::from(src), dest)).is_some() {
                        return Err(Error::Other("TODO: pkg with more than one EXE".into()));
                    }
                    continue;
                }
                files.push((PathBuf::from(src), dest));
                continue;
            }
            return Err(Error::Other(format!("TODO: pkg line: {line}")));
        }
        if !saw_en {
            return Err(Error::Other("pkg missing &EN".into()));
        }
        if !saw_platform {
            return Err(Error::Other("pkg missing platform UID 0x102752AE".into()));
        }
        let name = name.ok_or_else(|| Error::Other("pkg missing name".into()))?;
        let (exe, exe_dest) = exe.ok_or_else(|| Error::Other("pkg missing EXE".into()))?;
        let want = format!("{}{name}.exe", Self::EXE_DIR);
        if !exe_dest.eq_ignore_ascii_case(&want) {
            return Err(Error::Other(format!(
                "TODO: EXE dest other than {want}: {exe_dest}"
            )));
        }
        Ok(Self {
            name,
            uid3: uid3.ok_or_else(|| Error::Other("pkg missing UID".into()))?,
            version: version.ok_or_else(|| Error::Other("pkg missing version".into()))?,
            vendor: vendor.ok_or_else(|| Error::Other("pkg missing vendor".into()))?,
            vendor_localized: vendor_localized
                .ok_or_else(|| Error::Other("pkg missing localized vendor".into()))?,
            exe,
            files,
        })
    }

    const EXE_DIR: &str = "!:\\sys\\bin\\";

    fn is_exe_dest(dest: &str) -> bool {
        dest.len() > Self::EXE_DIR.len()
            && dest
                .get(..Self::EXE_DIR.len())
                .is_some_and(|dir| dir.eq_ignore_ascii_case(Self::EXE_DIR))
            && dest.to_ascii_lowercase().ends_with(".exe")
    }

    fn quoted(s: &str) -> Result<(String, &str)> {
        let s = s
            .strip_prefix('"')
            .ok_or_else(|| Error::Other("expected quoted string".into()))?;
        let end = s
            .find('"')
            .ok_or_else(|| Error::Other("unterminated quoted string".into()))?;
        Ok((s[..end].to_string(), &s[end + 1..]))
    }

    fn unquote(s: &str) -> Result<String> {
        let (v, rest) = Self::quoted(s)?;
        if !rest.is_empty() {
            return Err(Error::Other("unexpected trailing pkg text".into()));
        }
        Ok(v)
    }

    fn parse_uid3(tok: &str) -> Result<u32> {
        let hex = tok
            .strip_prefix("0x")
            .or_else(|| tok.strip_prefix("0X"))
            .ok_or_else(|| Error::Other(format!("invalid UID: {tok}")))?;
        u32::from_str_radix(hex, 16).map_err(|_| Error::Other(format!("invalid UID: {tok}")))
    }

    fn split_u32(s: &str) -> Result<(u32, &str)> {
        let (n, rest) = s
            .split_once(',')
            .ok_or_else(|| Error::Other(format!("expected number: {s}")))?;
        let n = n
            .trim()
            .parse()
            .map_err(|_| Error::Other(format!("invalid number: {n}")))?;
        Ok((n, rest.trim_start()))
    }
}
