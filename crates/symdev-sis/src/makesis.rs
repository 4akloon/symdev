use std::path::{Path, PathBuf};

use super::{SisDateTime, SisUnsigned, SisUnsignedSpec};
use symdev_core::{Error, Result};

#[derive(Debug)]
pub struct Makesis {
    pub verbose: bool,
    pub pkg: PathBuf,
    pub sis: PathBuf,
}

struct Wave0Pkg {
    name: String,
    uid3: u32,
    version: (u32, u32, u32),
    vendor: String,
    vendor_localized: String,
    exe: PathBuf,
    reg_rsc: Option<PathBuf>,
}

impl Makesis {
    pub fn from_args(args: &[String]) -> Result<Self> {
        let tokens = Self::skip_argv0(args);
        let mut verbose = false;
        let mut positional = Vec::new();
        for tok in tokens {
            match tok.as_str() {
                "-v" => verbose = true,
                // TODO: makesis -h -i -s -d directory (recorded usage; not Wave 0)
                "-h" | "-i" | "-s" => {
                    return Err(Error::Other(format!("TODO: makesis {tok}")));
                }
                "-d" => return Err(Error::Other("TODO: makesis -d directory".into())),
                s if s.starts_with('-') => {
                    return Err(Error::Other(format!("unknown makesis flag: {s}")));
                }
                s => positional.push(s.to_string()),
            }
        }
        match positional.as_slice() {
            [pkg, sis] => Ok(Self {
                verbose,
                pkg: PathBuf::from(pkg),
                sis: PathBuf::from(sis),
            }),
            [pkg] => {
                let pkg = PathBuf::from(pkg);
                let sis = pkg.with_extension("sis");
                Ok(Self { verbose, pkg, sis })
            }
            _ => Err(Error::Other("Usage: makesis [-v] pkgfile [sisfile]".into())),
        }
    }

    pub fn run(&self) -> Result<()> {
        let text = std::fs::read_to_string(&self.pkg)
            .map_err(|e| Error::Other(format!("read pkg {:?}: {e}", self.pkg)))?;
        let parsed = Wave0Pkg::parse(&text)?;
        let dir = self.pkg.parent().unwrap_or_else(|| Path::new("."));
        let exe_path = dir.join(&parsed.exe);
        let exe = std::fs::read(&exe_path)
            .map_err(|e| Error::Other(format!("read exe {exe_path:?}: {e}")))?;
        let _ = self.verbose;
        let rsc = match &parsed.reg_rsc {
            Some(src) => {
                let path = dir.join(src);
                Some(
                    std::fs::read(&path)
                        .map_err(|e| Error::Other(format!("read reg rsc {path:?}: {e}")))?,
                )
            }
            None => None,
        };
        // TODO: capability bits from E32 (Wine makesis; not in Wave 0 .pkg)
        let spec = SisUnsignedSpec {
            name: &parsed.name,
            uid3: parsed.uid3,
            version: parsed.version,
            vendor: &parsed.vendor,
            vendor_localized: &parsed.vendor_localized,
            exe: &exe,
            capabilities: &[],
            datetime: SisDateTime::utc(std::time::SystemTime::now()),
            reg_rsc: rsc.as_deref(),
        };
        let bytes = SisUnsigned::encode(&spec)?;
        std::fs::write(&self.sis, bytes)
            .map_err(|e| Error::Other(format!("write sis {:?}: {e}", self.sis)))?;
        Ok(())
    }

    fn skip_argv0(args: &[String]) -> &[String] {
        match args.first().map(String::as_str) {
            Some(s) if !s.starts_with('-') && !s.ends_with(".pkg") => &args[1..],
            _ => args,
        }
    }
}

impl Wave0Pkg {
    fn parse(text: &str) -> Result<Self> {
        let mut name = None;
        let mut uid3 = None;
        let mut version = None;
        let mut vendor_localized = None;
        let mut vendor = None;
        let mut exe = None;
        let mut reg_rsc = None;
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
                if Self::is_reg_rsc_dest(&dest) {
                    if reg_rsc.replace((PathBuf::from(src), dest)).is_some() {
                        return Err(Error::Other("pkg has more than one _reg.rsc".into()));
                    }
                    continue;
                }
                if exe.is_some() {
                    return Err(Error::Other(
                        "TODO: pkg files other than one EXE (rsc/mif)".into(),
                    ));
                }
                exe = Some(PathBuf::from(src));
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
        let reg_rsc = match reg_rsc {
            Some((src, dest)) => {
                let want = format!("{}{name}_reg.rsc", Self::REG_RSC_DIR);
                if !dest.eq_ignore_ascii_case(&want) {
                    return Err(Error::Other(format!(
                        "TODO: reg rsc dest other than {want}: {dest}"
                    )));
                }
                Some(src)
            }
            None => None,
        };
        Ok(Self {
            name,
            uid3: uid3.ok_or_else(|| Error::Other("pkg missing UID".into()))?,
            version: version.ok_or_else(|| Error::Other("pkg missing version".into()))?,
            vendor: vendor.ok_or_else(|| Error::Other("pkg missing vendor".into()))?,
            vendor_localized: vendor_localized
                .ok_or_else(|| Error::Other("pkg missing localized vendor".into()))?,
            exe: exe.ok_or_else(|| Error::Other("pkg missing EXE".into()))?,
            reg_rsc,
        })
    }

    const REG_RSC_DIR: &str = "!:\\private\\10003a3f\\import\\apps\\";

    fn is_reg_rsc_dest(dest: &str) -> bool {
        dest.len() > Self::REG_RSC_DIR.len()
            && dest
                .get(..Self::REG_RSC_DIR.len())
                .is_some_and(|dir| dir.eq_ignore_ascii_case(Self::REG_RSC_DIR))
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

#[cfg(test)]
mod tests {
    use super::*;

    fn args(tokens: &[&str]) -> Vec<String> {
        tokens.iter().map(|t| (*t).to_string()).collect()
    }

    #[test]
    fn from_args_match_experiment_7() {
        let m = Makesis::from_args(&args(&["makesis", "-v", "hello.pkg", "hello.sis"])).unwrap();
        assert!(m.verbose);
        assert_eq!(m.pkg, PathBuf::from("hello.pkg"));
        assert_eq!(m.sis, PathBuf::from("hello.sis"));
    }

    #[test]
    fn from_args_todo_unused_flags() {
        let err =
            Makesis::from_args(&args(&["makesis", "-i", "hello.pkg", "hello.sis"])).unwrap_err();
        assert!(err.to_string().contains("TODO: makesis -i"));
    }

    #[test]
    fn parse_wave0_pkg_matches_experiment_7() {
        let text = "&EN\r\n#{\"hello\"},(0xe79e4cf9),1,0,24,TYPE=SA\r\n%{\"Vendor-EN\"}\r\n:\"Vendor\"\r\n[0x102752AE], 0, 0, 0, {\"S60ProductID\"}\r\n\"hello.exe\"\t\t-\"!:\\sys\\bin\\hello.exe\"\r\n";
        let p = Wave0Pkg::parse(text).unwrap();
        assert_eq!(p.name, "hello");
        assert_eq!(p.uid3, 0xe79e_4cf9);
        assert_eq!(p.version, (1, 0, 24));
        assert_eq!(p.vendor, "Vendor");
        assert_eq!(p.vendor_localized, "Vendor-EN");
        assert_eq!(p.exe, PathBuf::from("hello.exe"));
        assert_eq!(p.reg_rsc, None);
    }

    #[test]
    fn parse_wave0_pkg_skips_comment_lines() {
        let text = "; header\r\n&EN\r\n; vendor\r\n#{\"hello\"},(0xe79e4cf9),1,0,24,TYPE=SA\r\n%{\"Vendor-EN\"}\r\n:\"Vendor\"\r\n[0x102752AE], 0, 0, 0, {\"S60ProductID\"}\r\n; EXEs\r\n\"hello.exe\"\t\t-\"!:\\sys\\bin\\hello.exe\"\r\n";
        let p = Wave0Pkg::parse(text).unwrap();
        assert_eq!(p.name, "hello");
        assert_eq!(p.exe, PathBuf::from("hello.exe"));
    }

    #[test]
    fn parse_wave0_pkg_rejects_reg_rsc_for_other_app() {
        let text = "&EN\n#{\"hello\"},(0xe79e4cf9),0,1,0,TYPE=SA\n%{\"symdev\"}\n:\"symdev\"\n[0x102752AE], 0, 0, 0, {\"S60ProductID\"}\n\"hello.exe\"\t\t-\"!:\\sys\\bin\\hello.exe\"\n\"other_reg.rsc\"\t\t-\"!:\\private\\10003a3f\\import\\apps\\other_reg.rsc\"\n";
        let err = Wave0Pkg::parse(text)
            .err()
            .map(|e| e.to_string())
            .unwrap_or_default();
        assert!(err.contains("reg rsc dest"), "{err}");
    }

    #[test]
    fn parse_wave0_pkg_accepts_verified_reg_rsc_dest() {
        let text = "&EN\n#{\"hello\"},(0xe79e4cf9),0,1,0,TYPE=SA\n%{\"symdev\"}\n:\"symdev\"\n[0x102752AE], 0, 0, 0, {\"S60ProductID\"}\n\"hello.exe\"\t\t-\"!:\\sys\\bin\\hello.exe\"\n\"hello_reg.rsc\"\t\t-\"!:\\private\\10003a3f\\import\\apps\\hello_reg.rsc\"\n";
        let p = Wave0Pkg::parse(text).unwrap();
        assert_eq!(p.exe, PathBuf::from("hello.exe"));
        assert_eq!(p.reg_rsc, Some(PathBuf::from("hello_reg.rsc")));
    }

    #[test]
    fn run_writes_unsigned_sis() {
        let dir = tempfile::tempdir().unwrap();
        let pkg = dir.path().join("hello.pkg");
        let sis = dir.path().join("hello.sis");
        std::fs::write(dir.path().join("hello.exe"), b"exe").unwrap();
        std::fs::write(
            &pkg,
            "&EN\n#{\"hello\"},(0xe79e4cf9),0,1,0,TYPE=SA\n%{\"symdev\"}\n:\"symdev\"\n[0x102752AE], 0, 0, 0, {\"S60ProductID\"}\n\"hello.exe\"\t\t-\"!:\\sys\\bin\\hello.exe\"\n",
        )
        .unwrap();
        Makesis {
            verbose: true,
            pkg,
            sis: sis.clone(),
        }
        .run()
        .unwrap();
        let bytes = std::fs::read(&sis).unwrap();
        assert_eq!(&bytes[..16], &crate::SisUid::new(0xe79e_4cf9).bytes());
    }
}
