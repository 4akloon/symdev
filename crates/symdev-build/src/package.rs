use std::path::{Path, PathBuf};
use std::time::SystemTime;

use symdev_core::{Artifact, Error, Package, PackageBackend, Result};
use symdev_makekeys::SelfSignedDsa;
use symdev_sis::{SisDateTime, SisPkgFile, SisUnsigned, SisUnsignedSpec};

pub struct SisPackage {
    pub name: String,
    /// The application binary's name (`AppTarget`): `build/<app>.exe` and the
    /// registration resource are named after it, not after `name`.
    pub app: String,
    pub uid3: u32,
    pub version: (u32, u32, u32),
    pub vendor: String,
    pub capabilities: Vec<String>,
    pub password: String,
    pub cert: Option<PathBuf>,
    pub key: Option<PathBuf>,
    /// Subject for a generated self-signed cert (`signing.subject`).
    pub subject: Option<String>,
}

impl SisPackage {
    /// `.pkg` text: header, the EXE, then `(source file, destination)` lines in order.
    pub fn pkg_text(&self, files: &[(String, String)]) -> String {
        let (major, minor, patch) = self.version;
        let mut text = format!(
            "&EN\r\n#{{\"{name}\"}},(0x{uid3:08x}),{major},{minor},{patch},TYPE=SA\r\n%{{\"{vendor}\"}}\r\n:\"{vendor}\"\r\n[0x102752AE], 0, 0, 0, {{\"S60ProductID\"}}\r\n\"{app}.exe\"\t\t-\"!:\\sys\\bin\\{app}.exe\"\r\n",
            name = self.name,
            app = self.app,
            uid3 = self.uid3,
            vendor = self.vendor,
        );
        for (src, dest) in files {
            text.push_str(&format!("\"{src}\"\t\t-\"{dest}\"\r\n"));
        }
        text
    }

    pub fn write_pkg_file(&self, dir: &Path, files: &[(String, String)]) -> Result<PathBuf> {
        let path = dir.join(format!("{}.pkg", self.name));
        std::fs::write(&path, self.pkg_text(files)).map_err(|e| Error::Other(e.to_string()))?;
        Ok(path)
    }

    pub fn validate_password(&self) -> Result<()> {
        if self.password.len() < 4 {
            return Err(Error::Other(
                "SYMDEV_SIGN_PASSWORD must be at least 4 characters".into(),
            ));
        }
        Ok(())
    }

    pub fn existing_signing_pair(&self) -> Option<(PathBuf, PathBuf)> {
        match (&self.cert, &self.key) {
            (Some(c), Some(k)) if c.is_file() && k.is_file() => Some((c.clone(), k.clone())),
            _ => None,
        }
    }
}

impl PackageBackend for SisPackage {
    fn package(&self, artifacts: &[Artifact]) -> Result<Package> {
        self.validate_password()?;
        let artifact = match artifacts
            .iter()
            .filter(|a| a.dest.is_none())
            .collect::<Vec<_>>()
            .as_slice()
        {
            [one] => *one,
            _ => return Err(Error::Other("no E32 artifact".into())),
        };
        let expected = format!("{}.exe", self.app);
        let name_ok = artifact
            .path
            .file_name()
            .is_some_and(|n| n == expected.as_str());
        if !name_ok || !artifact.path.is_file() {
            return Err(Error::Other("E32 not found".into()));
        }
        let workdir = artifact
            .path
            .parent()
            .ok_or_else(|| Error::Other("E32 not found".into()))?;
        let sis = format!("{}.sis", self.name);
        let sisx = format!("{}.sisx", self.name);
        let exe = std::fs::read(&artifact.path).map_err(|e| Error::Other(e.to_string()))?;
        let now = SystemTime::now();
        let datetime = SisDateTime::utc(now);
        let reg_dest = SisPkgFile::reg_rsc_dest(&self.app);
        // (source as the `.pkg` names it, destination, bytes), in build order. A file
        // next to the EXE is named bare, as the SDK example packages do; one from
        // elsewhere in the project (an `[[install]]` entry) keeps its own path.
        let mut extra: Vec<(String, String, Vec<u8>)> = Vec::new();
        for a in artifacts {
            let Some(dest) = &a.dest else { continue };
            let file = if a.path.parent() == Some(workdir) {
                a.path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .ok_or_else(|| Error::Other(format!("bad artifact path {:?}", a.path)))?
                    .to_string()
            } else {
                a.path.display().to_string()
            };
            let data = std::fs::read(&a.path)
                .map_err(|e| Error::Other(format!("read {}: {e}", a.path.display())))?;
            extra.push((file, dest.clone(), data));
        }
        // No project _reg.rsc: generate the recorded registration (experiment 43).
        if !extra
            .iter()
            .any(|(_, d, _)| d.eq_ignore_ascii_case(&reg_dest))
        {
            let rsc = symdev_rcomp::Rsc::registration(self.uid3, &self.app)?.bytes()?;
            let file = format!("{}_reg.rsc", self.app);
            std::fs::write(workdir.join(&file), &rsc).map_err(|e| Error::Other(e.to_string()))?;
            extra.push((file, reg_dest.clone(), rsc));
        }
        let lines: Vec<(String, String)> = extra
            .iter()
            .map(|(f, d, _)| (f.clone(), d.clone()))
            .collect();
        self.write_pkg_file(workdir, &lines)?;
        let files: Vec<SisPkgFile> = extra
            .iter()
            .map(|(_, dest, data)| SisPkgFile { dest, data })
            .collect();
        let spec = SisUnsignedSpec {
            name: &self.name,
            exe_name: &self.app,
            uid3: self.uid3,
            version: self.version,
            vendor: &self.vendor,
            vendor_localized: &self.vendor,
            exe: &exe,
            capabilities: &self.capabilities,
            datetime,
            files: &files,
        };
        let bytes = SisUnsigned::encode(&spec)?;
        std::fs::write(workdir.join(&sis), &bytes).map_err(|e| Error::Other(e.to_string()))?;
        let (cert_bytes, key_bytes) = match self.existing_signing_pair() {
            Some((cer, key)) => (
                std::fs::read(&cer).map_err(|e| Error::Other(e.to_string()))?,
                std::fs::read(&key).map_err(|e| Error::Other(e.to_string()))?,
            ),
            None => {
                let generated = match &self.subject {
                    Some(subject) => SelfSignedDsa::generate_for(subject, now)?,
                    None => SelfSignedDsa::generate(now)?,
                };
                std::fs::write(
                    workdir.join(format!("{}.cer", self.name)),
                    generated.cert_pem(),
                )
                .map_err(|e| Error::Other(e.to_string()))?;
                std::fs::write(
                    workdir.join(format!("{}.key", self.name)),
                    generated.key_pem(),
                )
                .map_err(|e| Error::Other(e.to_string()))?;
                (generated.cert_pem().to_vec(), generated.key_pem().to_vec())
            }
        };
        let sisx_bytes =
            SisUnsigned::encode_signed(&spec, &key_bytes, &cert_bytes, &self.password)?;
        std::fs::write(workdir.join(&sisx), sisx_bytes).map_err(|e| Error::Other(e.to_string()))?;
        Ok(Package {
            primary: workdir.join(&sisx),
            companions: vec![workdir.join(&sis)],
        })
    }
}

#[cfg(test)]
mod tests;
