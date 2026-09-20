//! `Makesis`: the `makesis.exe` CLI (Wave-0 `.pkg` to unsigned SIS).
use std::path::{Path, PathBuf};

use symdev_core::{Error, Result};

use crate::{SisDateTime, SisPkgFile, SisUnsigned, SisUnsignedSpec};
use wave0_pkg::Wave0Pkg;

mod wave0_pkg;

#[derive(Debug)]
pub struct Makesis {
    pub verbose: bool,
    pub pkg: PathBuf,
    pub sis: PathBuf,
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
        let mut contents = Vec::new();
        for (src, _) in &parsed.files {
            let path = dir.join(src);
            contents.push(
                std::fs::read(&path).map_err(|e| Error::Other(format!("read {path:?}: {e}")))?,
            );
        }
        let files: Vec<SisPkgFile> = parsed
            .files
            .iter()
            .zip(&contents)
            .map(|((_, dest), data)| SisPkgFile { dest, data })
            .collect();
        // TODO: capability bits from E32 (Wine makesis; not in Wave 0 .pkg)
        let exe_name = parsed
            .exe
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| Error::Other(format!("bad EXE name in .pkg: {}", parsed.exe.display())))?
            .to_string();
        let spec = SisUnsignedSpec {
            name: &parsed.name,
            exe_name: &exe_name,
            uid3: parsed.uid3,
            version: parsed.version,
            vendor: &parsed.vendor,
            vendor_localized: &parsed.vendor_localized,
            exe: &exe,
            capabilities: &[],
            datetime: SisDateTime::utc(std::time::SystemTime::now()),
            files: &files,
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

#[cfg(test)]
mod tests;
