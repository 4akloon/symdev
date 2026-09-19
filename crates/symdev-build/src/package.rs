use std::path::{Path, PathBuf};
use std::time::SystemTime;

use symdev_core::{Artifact, Error, Package, PackageBackend, Result};
use symdev_makekeys::SelfSignedDsa;
use symdev_sis::{SisDateTime, SisUnsigned, SisUnsignedSpec};

pub struct SisPackage {
    pub name: String,
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
    pub fn pkg_text(&self) -> String {
        let (major, minor, patch) = self.version;
        format!(
            "&EN\r\n#{{\"{name}\"}},(0x{uid3:08x}),{major},{minor},{patch},TYPE=SA\r\n%{{\"{vendor}\"}}\r\n:\"{vendor}\"\r\n[0x102752AE], 0, 0, 0, {{\"S60ProductID\"}}\r\n\"{name}.exe\"\t\t-\"!:\\sys\\bin\\{name}.exe\"\r\n\"{name}_reg.rsc\"\t\t-\"!:\\private\\10003a3f\\import\\apps\\{name}_reg.rsc\"\r\n",
            name = self.name,
            uid3 = self.uid3,
            vendor = self.vendor,
        )
    }

    pub fn write_pkg_file(&self, dir: &Path) -> Result<PathBuf> {
        let path = dir.join(format!("{}.pkg", self.name));
        std::fs::write(&path, self.pkg_text()).map_err(|e| Error::Other(e.to_string()))?;
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
        let artifact = match artifacts {
            [one] => one,
            _ => return Err(Error::Other("no E32 artifact".into())),
        };
        let expected = format!("{}.exe", self.name);
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
        self.write_pkg_file(workdir)?;
        let sis = format!("{}.sis", self.name);
        let sisx = format!("{}.sisx", self.name);
        let exe = std::fs::read(&artifact.path).map_err(|e| Error::Other(e.to_string()))?;
        let now = SystemTime::now();
        let datetime = SisDateTime::utc(now);
        let rsc = symdev_rcomp::Rsc::registration(self.uid3, &self.name)?.bytes()?;
        std::fs::write(workdir.join(format!("{}_reg.rsc", self.name)), &rsc)
            .map_err(|e| Error::Other(e.to_string()))?;
        let spec = SisUnsignedSpec {
            name: &self.name,
            uid3: self.uid3,
            version: self.version,
            vendor: &self.vendor,
            vendor_localized: &self.vendor,
            exe: &exe,
            capabilities: &self.capabilities,
            datetime,
            reg_rsc: Some(&rsc),
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

#[test]
fn dname_is_makekeys_example_usage() {
    assert_eq!(
        SelfSignedDsa::DNAME,
        "CN=Joe Bloggs OU=Development O=Acme Ltd C=GB EM=noone@nowhere.com"
    );
}

#[test]
fn validate_password_rejects_shorter_than_four_characters() {
    let mut pkg = fake_pkg();
    pkg.password = "abc".into();
    let err = pkg.validate_password().unwrap_err();
    assert_eq!(
        err.to_string(),
        "SYMDEV_SIGN_PASSWORD must be at least 4 characters"
    );
    pkg.password = "".into();
    assert!(pkg.validate_password().is_err());
    pkg.password = "abcd".into();
    pkg.validate_password().unwrap();
}

#[test]
fn write_pkg_file_writes_recorded_pkg_next_to_exe() {
    let dir = tempfile::tempdir().unwrap();
    let path = fake_pkg().write_pkg_file(dir.path()).unwrap();
    assert_eq!(path, dir.path().join("hello.pkg"));
    let s = std::fs::read_to_string(&path).unwrap();
    assert!(s.contains("\r\n"));
    assert_eq!(
        s.replace("\r\n", "\n"),
        "&EN\n#{\"hello\"},(0xe79e4cf9),0,1,0,TYPE=SA\n%{\"symdev\"}\n:\"symdev\"\n[0x102752AE], 0, 0, 0, {\"S60ProductID\"}\n\"hello.exe\"\t\t-\"!:\\sys\\bin\\hello.exe\"\n\"hello_reg.rsc\"\t\t-\"!:\\private\\10003a3f\\import\\apps\\hello_reg.rsc\"\n"
    );
}

#[test]
fn pkg_text_includes_verified_reg_rsc_dest() {
    let s = fake_pkg().pkg_text();
    assert!(s.contains("\r\n"));
    assert_eq!(
        s.replace("\r\n", "\n"),
        "&EN\n#{\"hello\"},(0xe79e4cf9),0,1,0,TYPE=SA\n%{\"symdev\"}\n:\"symdev\"\n[0x102752AE], 0, 0, 0, {\"S60ProductID\"}\n\"hello.exe\"\t\t-\"!:\\sys\\bin\\hello.exe\"\n\"hello_reg.rsc\"\t\t-\"!:\\private\\10003a3f\\import\\apps\\hello_reg.rsc\"\n"
    );
}

#[cfg(test)]
fn fake_pkg() -> SisPackage {
    SisPackage {
        name: "hello".into(),
        uid3: 0xe79e4cf9,
        version: (0, 1, 0),
        vendor: "symdev".into(),
        capabilities: Vec::new(),
        password: "secret".into(),
        cert: None,
        key: None,
        subject: None,
    }
}

#[test]
fn package_empty_artifacts_is_no_e32_artifact() {
    let err = fake_pkg().package(&[]).unwrap_err();
    assert_eq!(err.to_string(), "no E32 artifact");
}

#[test]
fn package_missing_e32_file_errors() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("hello.exe");
    let err = fake_pkg().package(&[Artifact { path }]).unwrap_err();
    assert_eq!(err.to_string(), "E32 not found");
}

#[test]
fn package_short_password_errors_before_tools() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("hello.exe");
    std::fs::write(&path, b"").unwrap();
    let mut pkg = fake_pkg();
    pkg.password = "abc".into();
    let err = pkg.package(&[Artifact { path }]).unwrap_err();
    assert_eq!(
        err.to_string(),
        "SYMDEV_SIGN_PASSWORD must be at least 4 characters"
    );
}

#[test]
fn existing_signing_pair_when_both_files_exist() {
    let dir = tempfile::tempdir().unwrap();
    let cert = dir.path().join("hello.cer");
    let key = dir.path().join("hello.key");
    std::fs::write(&cert, b"c").unwrap();
    std::fs::write(&key, b"k").unwrap();
    let mut pkg = fake_pkg();
    pkg.cert = Some(cert.clone());
    pkg.key = Some(key.clone());
    assert_eq!(pkg.existing_signing_pair(), Some((cert, key)));
}

#[test]
fn existing_signing_pair_none_when_missing() {
    let dir = tempfile::tempdir().unwrap();
    let cert = dir.path().join("hello.cer");
    let key = dir.path().join("hello.key");
    std::fs::write(&cert, b"c").unwrap();
    let mut pkg = fake_pkg();
    pkg.cert = Some(cert.clone());
    pkg.key = Some(key);
    assert_eq!(pkg.existing_signing_pair(), None);
    pkg.cert = None;
    pkg.key = None;
    assert_eq!(pkg.existing_signing_pair(), None);
    pkg.cert = Some(cert);
    pkg.key = None;
    assert_eq!(pkg.existing_signing_pair(), None);
}

#[cfg(test)]
fn parse_hex(s: &str) -> Vec<u8> {
    let hex: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
        .collect()
}

#[cfg(test)]
fn hello_sis_golden() -> Vec<u8> {
    parse_hex(include_str!("../../symdev-sis/src/testdata/hello_sis.hex"))
}

#[cfg(test)]
fn hello_exe_bytes() -> Vec<u8> {
    parse_hex(include_str!(
        "../../symdev-sis/src/testdata/hello_type30.hex"
    ))[60..]
        .to_vec()
}

#[cfg(test)]
fn hello_datetime() -> symdev_sis::SisDateTime {
    symdev_sis::SisDateTime::new(
        symdev_sis::SisDate::new(2026, 8, 17),
        symdev_sis::SisTime::new(15, 18, 24),
    )
}

#[cfg(test)]
fn hello_caps() -> Vec<String> {
    [
        "LocalServices",
        "NetworkServices",
        "ReadUserData",
        "WriteUserData",
        "UserEnvironment",
        "Location",
    ]
    .into_iter()
    .map(str::to_string)
    .collect()
}

#[test]
fn package_writes_native_sis_and_keys_without_wine() {
    let dir = tempfile::tempdir().unwrap();
    let exe_path = dir.path().join("hello.exe");
    std::fs::write(&exe_path, hello_exe_bytes()).unwrap();
    let stale_cer = dir.path().join("hello.cer");
    let stale_key = dir.path().join("hello.key");
    std::fs::write(&stale_cer, b"stale-cer").unwrap();
    std::fs::write(&stale_key, b"stale-key").unwrap();
    let mut pkg = fake_pkg();
    pkg.uid3 = 0xe79e_4cf9;
    pkg.version = (1, 0, 24);
    pkg.vendor = "Vendor".into();
    pkg.capabilities = hello_caps();
    let out = pkg
        .package(&[Artifact { path: exe_path }])
        .expect("native makekeys must not spawn Wine when cert/key are absent");
    assert_eq!(out.primary.file_name().unwrap(), "hello.sisx");
    assert!(dir.path().join("hello.sis").is_file());
    assert!(dir.path().join("hello.sisx").is_file());
    assert!(dir.path().join("hello.pkg").is_file());
    let rsc = dir.path().join("hello_reg.rsc");
    assert!(rsc.is_file());
    let rsc_bytes = std::fs::read(&rsc).unwrap();
    assert_eq!(
        &rsc_bytes[..16],
        &symdev_rcomp::RscUid::registration(0xe79e_4cf9).bytes()
    );
    let sis = std::fs::read(dir.path().join("hello.sis")).unwrap();
    let stored = symdev_sis::SisCompressed::smallest(&rsc_bytes)
        .unwrap()
        .data;
    assert!(sis.windows(stored.len()).any(|w| w == stored));
    let cer = std::fs::read(&stale_cer).unwrap();
    let key = std::fs::read(&stale_key).unwrap();
    assert_ne!(cer, b"stale-cer");
    assert_ne!(key, b"stale-key");
    let cer_text = String::from_utf8_lossy(&cer);
    let key_text = String::from_utf8_lossy(&key);
    assert!(cer_text.contains("BEGIN CERTIFICATE"), "{cer_text}");
    assert!(key_text.contains("BEGIN PRIVATE KEY"), "{key_text}");
}

#[test]
fn package_writes_native_sisx_without_wine_signsis() {
    let dir = tempfile::tempdir().unwrap();
    let exe_path = dir.path().join("hello.exe");
    std::fs::write(&exe_path, hello_exe_bytes()).unwrap();
    let cert = dir.path().join("hello.cer");
    let key = dir.path().join("hello.key");
    std::fs::write(
        &cert,
        parse_hex(include_str!(
            "../../symdev-sis/src/testdata/test_dsa_cert.hex"
        )),
    )
    .unwrap();
    std::fs::write(
        &key,
        parse_hex(include_str!(
            "../../symdev-sis/src/testdata/test_dsa_key.hex"
        )),
    )
    .unwrap();
    let mut pkg = fake_pkg();
    pkg.uid3 = 0xe79e_4cf9;
    pkg.version = (1, 0, 24);
    pkg.vendor = "Vendor".into();
    pkg.capabilities = hello_caps();
    pkg.cert = Some(cert);
    pkg.key = Some(key);
    let out = pkg
        .package(&[Artifact { path: exe_path }])
        .expect("native SISX must not spawn Wine signsis");
    assert!(out.primary.is_file());
    assert_eq!(out.primary.file_name().unwrap(), "hello.sisx");
    assert!(dir.path().join("hello.sis").is_file());
    assert!(dir.path().join("hello.sisx").is_file());
    let sisx = std::fs::read(&out.primary).unwrap();
    assert_eq!(&sisx[..16], &symdev_sis::SisUid::new(0xe79e_4cf9).bytes());
    assert_ne!(sisx, hello_sis_golden());
    assert!(sisx.len() > hello_sis_golden().len());
}

#[cfg(test)]
fn hello_makekeys_not_before() -> SystemTime {
    // Frozen experiment-8 hello.cer Not Before: 2026-09-17 15:21:21 GMT
    std::time::UNIX_EPOCH + std::time::Duration::from_secs(1_789_654_881)
}

#[test]
fn generated_self_signed_dsa_verifies_with_injected_dates() {
    use der::{Decode, DecodePem, Encode};
    let not_before = hello_makekeys_not_before();
    let generated = SelfSignedDsa::generate(not_before).unwrap();
    let cert_pem = generated.cert_pem();
    let key_pem = generated.key_pem();
    let cert = x509_cert::Certificate::from_pem(&cert_pem).unwrap();
    assert_eq!(
        cert.tbs_certificate.serial_number,
        x509_cert::serial_number::SerialNumber::from(1u32)
    );
    assert_eq!(
        cert.tbs_certificate.signature.oid.to_string(),
        "1.2.840.10040.4.3"
    );
    let subject = cert.tbs_certificate.subject.to_string();
    assert!(subject.contains("Joe Bloggs"), "{subject}");
    assert!(subject.contains("Development"), "{subject}");
    assert!(subject.contains("Acme Ltd"), "{subject}");
    assert!(subject.contains("GB"), "{subject}");
    assert!(subject.contains("noone@nowhere.com"), "{subject}");
    assert_eq!(
        cert.tbs_certificate.validity.not_before.to_unix_duration(),
        std::time::Duration::from_secs(1_789_654_881)
    );
    assert_eq!(
        cert.tbs_certificate.validity.not_after.to_unix_duration(),
        std::time::Duration::from_secs(1_789_654_881 + 3650 * 86400)
    );
    let der = x509_cert::Certificate::from_pem(&cert_pem)
        .unwrap()
        .to_der()
        .unwrap();
    let parsed = x509_cert::Certificate::from_der(&der).unwrap();
    symdev_sis::SisBlob37::new(parsed.signature.raw_bytes().to_vec())
        .verify_dsa_sha1(&parsed.tbs_certificate.to_der().unwrap(), &der)
        .unwrap();
    let spec = SisUnsignedSpec {
        name: "hello",
        uid3: 0xe79e_4cf9,
        version: (1, 0, 24),
        vendor: "Vendor",
        vendor_localized: "Vendor-EN",
        exe: &hello_exe_bytes(),
        capabilities: &hello_caps(),
        datetime: hello_datetime(),
        reg_rsc: None,
    };
    let sisx = SisUnsigned::encode_signed(&spec, &key_pem, &cert_pem, "").unwrap();
    assert!(sisx.len() > hello_sis_golden().len());
    let exp = std::path::PathBuf::from(std::env::var_os("HOME").unwrap_or_default())
        .join("src/symdev-experiment-5/hello.cer");
    if exp.is_file() {
        let frozen = std::fs::read(&exp).unwrap();
        assert_ne!(
            cert_pem, frozen,
            "dates/serial/k/key material block a hello.cer byte-match"
        );
    }
}
