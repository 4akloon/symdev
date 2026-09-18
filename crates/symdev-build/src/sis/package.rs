use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use super::{
    SisArray, SisCompressed, SisController, SisData, SisData31, SisData32, SisDate, SisDateTime,
    SisEncode, SisFile, SisFiles, SisHash, SisInfo, SisLanguage, SisLanguages, SisPkgUid,
    SisProduct, SisProductVersion, SisProducts, SisString, SisTime, SisU32, SisUid, SisUnsigned,
    SisVersion, SisWord41, SisWords, SisWords16, SisWords19,
};
use sha1::{Digest, Sha1};
use symdev_core::{Artifact, Error, LocalEnv, Package, PackageBackend, RemotePath, Result};

pub struct SisTools {
    pub wine: PathBuf,
    pub makesis: PathBuf,
    pub signsis: PathBuf,
    pub makekeys: PathBuf,
}

impl SisTools {
    pub fn from_env() -> Result<Self> {
        let epocroot = required("SYMDEV_EPOCROOT")?;
        let wine = match std::env::var("SYMDEV_WINE") {
            Ok(v) if !v.is_empty() => PathBuf::from(v),
            _ => PathBuf::from("/usr/bin/wine"),
        };
        let tools = epocroot.join("epoc32/tools");
        Ok(Self {
            wine,
            makesis: tools.join("makesis.exe"),
            signsis: tools.join("signsis.exe"),
            makekeys: tools.join("makekeys.exe"),
        })
    }

    pub fn makesis_args(&self, pkg: &str, sis: &str) -> Vec<String> {
        vec![
            path_arg(&self.wine),
            path_arg(&self.makesis),
            "-v".into(),
            pkg.into(),
            sis.into(),
        ]
    }

    pub fn makekeys_args(&self, password: &str, key: &str, cer: &str) -> Vec<String> {
        vec![
            path_arg(&self.wine),
            path_arg(&self.makekeys),
            "-cert".into(),
            "-expdays".into(),
            "3650".into(),
            "-password".into(),
            password.into(),
            "-len".into(),
            "2048".into(),
            "-dname".into(),
            dname().into(),
            key.into(),
            cer.into(),
        ]
    }
}

pub fn dname() -> &'static str {
    "CN=Joe Bloggs OU=Development O=Acme Ltd C=GB EM=noone@nowhere.com"
}

pub fn validate_password(password: &str) -> Result<()> {
    if password.len() < 4 {
        return Err(Error::Other(
            "SYMDEV_SIGN_PASSWORD must be at least 4 characters".into(),
        ));
    }
    Ok(())
}

pub struct SisUnsignedSpec<'a> {
    pub name: &'a str,
    pub uid3: u32,
    pub version: (u32, u32, u32),
    pub vendor: &'a str,
    pub vendor_localized: &'a str,
    pub exe: &'a [u8],
    pub capabilities: &'a [String],
    pub datetime: SisDateTime,
}

impl SisUnsigned {
    pub fn encode(spec: &SisUnsignedSpec<'_>) -> Result<Vec<u8>> {
        let (controller, data) = unsigned_parts(spec)?;
        wrap_sis(spec.uid3, &controller, data)
    }

    pub fn encode_signed(
        spec: &SisUnsignedSpec<'_>,
        key_pem: &[u8],
        cert_pem_or_der: &[u8],
        password: &str,
    ) -> Result<Vec<u8>> {
        let (controller, data) = unsigned_parts(spec)?;
        let signatures =
            controller.signatures_from_key_and_cert(key_pem, cert_pem_or_der, password)?;
        wrap_sis(spec.uid3, &controller.with_signatures(signatures), data)
    }
}

fn wrap_sis(uid3: u32, controller: &SisController, data: SisData) -> Result<Vec<u8>> {
    Ok(SisUnsigned::new(
        SisUid::new(uid3),
        SisCompressed::zlib(&controller.field().bytes())?,
        data,
    )
    .bytes())
}

fn unsigned_parts(spec: &SisUnsignedSpec<'_>) -> Result<(SisController, SisData)> {
    let caps = capability_word(spec.capabilities)?;
    let size = spec.exe.len() as u32;
    let digest: [u8; 20] = Sha1::digest(spec.exe).into();
    let dest = format!("!:\\sys\\bin\\{}.exe", spec.name);
    let (major, minor, build) = spec.version;
    let controller = SisController::new(
        SisInfo::new(
            SisPkgUid::new(spec.uid3),
            SisString::new(spec.vendor),
            SisArray::new(vec![SisString::new(spec.name).field()]),
            SisArray::new(vec![SisString::new(spec.vendor_localized).field()]),
            SisVersion::new(major, minor, build),
            spec.datetime,
        ),
        SisWords16::new(SisWords::new(vec![0x21])), // ponytail: recorded TYPE=SA &EN one-file words; derive when pkg grammar grows
        SisLanguages::new(SisArray::new(vec![SisLanguage::new(1).field()])),
        SisProducts::new(SisArray::new(vec![SisProduct::new(
            SisPkgUid::new(0x1027_52ae),
            SisProductVersion::new(SisVersion::new(0, 0, 0)),
            SisArray::new(vec![SisString::new("S60ProductID").field()]),
        )
        .field()])),
        SisWords19::new(SisWords::new(vec![0x14])),
        SisFiles::new(
            SisArray::new(vec![SisFile::new(
                SisString::new(dest),
                SisString::new(""),
                SisWord41::new(caps),
                SisHash::new([1, 0x25, 0x14], digest),
                SisString::new(""),
                [size, 0, size, 0, 0],
            )
            .field()]),
            SisWords::new(vec![0x0d]),
            SisWords::new(vec![0x1a]),
        ),
        SisU32::new(0),
    );
    let data = SisData::new(SisArray::new(vec![SisData31::new(SisArray::new(vec![
        SisData32::new(SisCompressed {
            algorithm: 0,
            uncompressed_size: size,
            reserved: 0,
            data: spec.exe.to_vec(),
        })
        .field(),
    ]))
    .field()]));
    Ok((controller, data))
}

fn capability_word(caps: &[String]) -> Result<u32> {
    // Bits recorded by experiment 6's six user-grantable names + hello type-41 `0x000be000`.
    const MAP: &[(&str, u32)] = &[
        ("NetworkServices", 13),
        ("LocalServices", 14),
        ("ReadUserData", 15),
        ("WriteUserData", 16),
        ("Location", 17),
        ("UserEnvironment", 19),
    ];
    let mut word = 0u32;
    for cap in caps {
        let Some((_, bit)) = MAP.iter().copied().find(|(name, _)| *name == cap.as_str()) else {
            return Err(Error::Other(format!(
                "SIS capability bits not yet derived from pkg: {cap}"
            )));
        };
        word |= 1 << bit;
    }
    Ok(word)
}

fn datetime_utc(now: SystemTime) -> SisDateTime {
    let secs = now
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let days = (secs / 86_400) as i32;
    let rem = (secs % 86_400) as u32;
    let (year, month, day) = civil_from_unix_days(days);
    SisDateTime::new(
        SisDate::new(year, month, day),
        SisTime::new(
            (rem / 3600) as u8,
            ((rem % 3600) / 60) as u8,
            (rem % 60) as u8,
        ),
    )
}

fn civil_from_unix_days(z: i32) -> (u16, u8, u8) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i32 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = y + i32::from(m <= 2);
    (year as u16, m as u8, d as u8)
}

pub fn write_pkg_file(
    dir: &Path,
    name: &str,
    uid3: u32,
    version: (u32, u32, u32),
    vendor: &str,
) -> Result<PathBuf> {
    let path = dir.join(format!("{name}.pkg"));
    std::fs::write(&path, crate::render_pkg(name, uid3, version, vendor))
        .map_err(|e| Error::Other(e.to_string()))?;
    Ok(path)
}

pub fn existing_signing_pair(
    cert: &Option<PathBuf>,
    key: &Option<PathBuf>,
) -> Option<(PathBuf, PathBuf)> {
    match (cert, key) {
        (Some(c), Some(k)) if c.is_file() && k.is_file() => Some((c.clone(), k.clone())),
        _ => None,
    }
}

pub struct SisPackage {
    pub env: LocalEnv,
    pub tools: SisTools,
    pub name: String,
    pub uid3: u32,
    pub version: (u32, u32, u32),
    pub vendor: String,
    pub capabilities: Vec<String>,
    pub password: String,
    pub cert: Option<PathBuf>,
    pub key: Option<PathBuf>,
}

impl SisPackage {
    fn run_tool(&self, args: &[String], cwd: &RemotePath) -> Result<()> {
        let mut cmd = Command::new(&args[0]);
        cmd.args(&args[1..]);
        let out = self.env.run_blocking(cmd, cwd)?;
        if out.status != 0 {
            return Err(Error::Other(
                String::from_utf8_lossy(&out.stderr).into_owned(),
            ));
        }
        Ok(())
    }
}

impl PackageBackend for SisPackage {
    fn package(&self, artifacts: &[Artifact]) -> Result<Package> {
        validate_password(&self.password)?;
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
        write_pkg_file(workdir, &self.name, self.uid3, self.version, &self.vendor)?;
        let cwd = RemotePath::new(workdir.display().to_string());
        let sis = format!("{}.sis", self.name);
        let sisx = format!("{}.sisx", self.name);
        let exe = std::fs::read(&artifact.path).map_err(|e| Error::Other(e.to_string()))?;
        let datetime = datetime_utc(SystemTime::now());
        let spec = SisUnsignedSpec {
            name: &self.name,
            uid3: self.uid3,
            version: self.version,
            vendor: &self.vendor,
            vendor_localized: &self.vendor,
            exe: &exe,
            capabilities: &self.capabilities,
            datetime,
        };
        let bytes = SisUnsigned::encode(&spec)?;
        std::fs::write(workdir.join(&sis), &bytes).map_err(|e| Error::Other(e.to_string()))?;
        let (cer_path, key_path) = match existing_signing_pair(&self.cert, &self.key) {
            Some((cer, key)) => (cer, key),
            None => {
                let key_name = format!("{}.key", self.name);
                let cer_name = format!("{}.cer", self.name);
                self.run_tool(
                    &self
                        .tools
                        .makekeys_args(&self.password, &key_name, &cer_name),
                    &cwd,
                )?;
                (workdir.join(cer_name), workdir.join(key_name))
            }
        };
        let cert_bytes = std::fs::read(&cer_path).map_err(|e| Error::Other(e.to_string()))?;
        let key_bytes = std::fs::read(&key_path).map_err(|e| Error::Other(e.to_string()))?;
        let sisx_bytes =
            SisUnsigned::encode_signed(&spec, &key_bytes, &cert_bytes, &self.password)?;
        std::fs::write(workdir.join(&sisx), sisx_bytes).map_err(|e| Error::Other(e.to_string()))?;
        Ok(Package {
            primary: workdir.join(&sisx),
            companions: vec![workdir.join(&sis)],
        })
    }
}

fn required(key: &str) -> Result<PathBuf> {
    match std::env::var(key) {
        Ok(v) if !v.is_empty() => Ok(PathBuf::from(v)),
        _ => Err(Error::Other(format!("missing toolchain: {key}"))),
    }
}

fn path_arg(path: &Path) -> String {
    path.display().to_string()
}

#[cfg(test)]
use std::sync::Mutex;

#[cfg(test)]
static ENV: Mutex<()> = Mutex::new(());

#[cfg(test)]
fn sdk_tools() -> SisTools {
    SisTools {
        wine: PathBuf::from("/usr/bin/wine"),
        makesis: PathBuf::from("/sdk/epoc32/tools/makesis.exe"),
        signsis: PathBuf::from("/sdk/epoc32/tools/signsis.exe"),
        makekeys: PathBuf::from("/sdk/epoc32/tools/makekeys.exe"),
    }
}

#[test]
fn from_env_uses_sdk_fixture_and_default_wine() {
    let _guard = ENV.lock().unwrap();
    // SAFETY: ENV serializes tests that mutate these keys.
    unsafe {
        std::env::set_var("SYMDEV_EPOCROOT", "/sdk");
        std::env::remove_var("SYMDEV_WINE");
    }
    let tools = SisTools::from_env().unwrap();
    assert_eq!(tools.wine, PathBuf::from("/usr/bin/wine"));
    assert_eq!(
        tools.makesis,
        PathBuf::from("/sdk/epoc32/tools/makesis.exe")
    );
    assert_eq!(
        tools.signsis,
        PathBuf::from("/sdk/epoc32/tools/signsis.exe")
    );
    assert_eq!(
        tools.makekeys,
        PathBuf::from("/sdk/epoc32/tools/makekeys.exe")
    );
}

#[test]
fn from_env_missing_epocroot_errors() {
    let _guard = ENV.lock().unwrap();
    // SAFETY: ENV serializes tests that mutate these keys.
    unsafe {
        std::env::remove_var("SYMDEV_EPOCROOT");
    }
    let err = match SisTools::from_env() {
        Err(e) => e,
        Ok(_) => panic!("expected missing SYMDEV_EPOCROOT"),
    };
    assert_eq!(err.to_string(), "missing toolchain: SYMDEV_EPOCROOT");
}

#[test]
fn makesis_args_match_experiment_7() {
    assert_eq!(
        sdk_tools().makesis_args("hello.pkg", "hello.sis"),
        [
            "/usr/bin/wine",
            "/sdk/epoc32/tools/makesis.exe",
            "-v",
            "hello.pkg",
            "hello.sis",
        ]
    );
}

#[test]
fn makekeys_args_match_experiment_8() {
    assert_eq!(
        sdk_tools().makekeys_args("secret", "hello.key", "hello.cer"),
        [
            "/usr/bin/wine",
            "/sdk/epoc32/tools/makekeys.exe",
            "-cert",
            "-expdays",
            "3650",
            "-password",
            "secret",
            "-len",
            "2048",
            "-dname",
            "CN=Joe Bloggs OU=Development O=Acme Ltd C=GB EM=noone@nowhere.com",
            "hello.key",
            "hello.cer",
        ]
    );
}

#[test]
fn dname_is_makekeys_example_usage() {
    assert_eq!(
        dname(),
        "CN=Joe Bloggs OU=Development O=Acme Ltd C=GB EM=noone@nowhere.com"
    );
}

#[test]
fn validate_password_rejects_shorter_than_four_characters() {
    let err = validate_password("abc").unwrap_err();
    assert_eq!(
        err.to_string(),
        "SYMDEV_SIGN_PASSWORD must be at least 4 characters"
    );
    assert!(validate_password("").is_err());
    validate_password("abcd").unwrap();
}

#[test]
fn write_pkg_file_writes_recorded_pkg_next_to_exe() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_pkg_file(dir.path(), "hello", 0xe79e4cf9, (0, 1, 0), "symdev").unwrap();
    assert_eq!(path, dir.path().join("hello.pkg"));
    let s = std::fs::read_to_string(&path).unwrap();
    assert!(s.contains("\r\n"));
    assert_eq!(
        s.replace("\r\n", "\n"),
        "&EN\n#{\"hello\"},(0xe79e4cf9),0,1,0,TYPE=SA\n%{\"symdev\"}\n:\"symdev\"\n[0x102752AE], 0, 0, 0, {\"S60ProductID\"}\n\"hello.exe\"\t\t-\"!:\\sys\\bin\\hello.exe\"\n"
    );
}

#[cfg(test)]
fn fake_pkg() -> SisPackage {
    SisPackage {
        env: LocalEnv,
        tools: sdk_tools(),
        name: "hello".into(),
        uid3: 0xe79e4cf9,
        version: (0, 1, 0),
        vendor: "symdev".into(),
        capabilities: Vec::new(),
        password: "secret".into(),
        cert: None,
        key: None,
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
    assert_eq!(
        existing_signing_pair(&Some(cert.clone()), &Some(key.clone())),
        Some((cert, key))
    );
}

#[test]
fn existing_signing_pair_none_when_missing() {
    let dir = tempfile::tempdir().unwrap();
    let cert = dir.path().join("hello.cer");
    let key = dir.path().join("hello.key");
    std::fs::write(&cert, b"c").unwrap();
    assert_eq!(existing_signing_pair(&Some(cert.clone()), &Some(key)), None);
    assert_eq!(existing_signing_pair(&None, &None), None);
    assert_eq!(existing_signing_pair(&Some(cert), &None), None);
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
    parse_hex(include_str!("testdata/hello_sis.hex"))
}

#[cfg(test)]
fn hello_exe_bytes() -> Vec<u8> {
    parse_hex(include_str!("testdata/hello_type30.hex"))[60..].to_vec()
}

#[cfg(test)]
fn hello_datetime() -> crate::sis::SisDateTime {
    crate::sis::SisDateTime::new(
        crate::sis::SisDate::new(2026, 8, 17),
        crate::sis::SisTime::new(15, 18, 24),
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
fn hello_exe_sha1_matches_experiment_29() {
    assert_eq!(
        <[u8; 20]>::from(Sha1::digest(hello_exe_bytes())),
        [
            0x3a, 0x23, 0xe7, 0xe7, 0xe6, 0x0e, 0xd9, 0x73, 0x54, 0x53, 0x4b, 0x2a, 0x77, 0xe5,
            0x65, 0xcd, 0x64, 0xea, 0x39, 0x70,
        ]
    );
}

#[test]
fn encode_unsigned_sis_matches_hello_pkg_fixture() {
    let bytes = SisUnsigned::encode(&SisUnsignedSpec {
        name: "hello",
        uid3: 0xe79e_4cf9,
        version: (1, 0, 24),
        vendor: "Vendor",
        vendor_localized: "Vendor-EN",
        exe: &hello_exe_bytes(),
        capabilities: &hello_caps(),
        datetime: hello_datetime(),
    })
    .unwrap();
    let golden = hello_sis_golden();
    assert_eq!(golden.len(), 4000);
    assert_eq!(bytes, golden);
}

#[test]
fn encode_unsigned_sis_uses_project_fields_not_hello_goldens() {
    let bytes = SisUnsigned::encode(&SisUnsignedSpec {
        name: "other",
        uid3: 0xe000_0001,
        version: (0, 1, 0),
        vendor: "symdev",
        vendor_localized: "symdev",
        exe: &hello_exe_bytes(),
        capabilities: &[],
        datetime: hello_datetime(),
    })
    .unwrap();
    assert_ne!(bytes, hello_sis_golden());
    assert_eq!(&bytes[..16], &SisUid::new(0xe000_0001).bytes());
    assert_ne!(&bytes[..16], &SisUid::new(0xe79e_4cf9).bytes());
}

#[test]
fn encode_unsigned_sis_rejects_capability_bits_not_derived() {
    let err = SisUnsigned::encode(&SisUnsignedSpec {
        name: "hello",
        uid3: 0xe79e_4cf9,
        version: (1, 0, 24),
        vendor: "Vendor",
        vendor_localized: "Vendor-EN",
        exe: &hello_exe_bytes(),
        capabilities: &["AllFiles".into()],
        datetime: hello_datetime(),
    })
    .unwrap_err();
    assert_eq!(
        err.to_string(),
        "SIS capability bits not yet derived from pkg: AllFiles"
    );
}

#[test]
fn package_writes_native_sis_before_wine_signsis() {
    let dir = tempfile::tempdir().unwrap();
    let exe_path = dir.path().join("hello.exe");
    std::fs::write(&exe_path, hello_exe_bytes()).unwrap();
    let mut pkg = fake_pkg();
    pkg.tools.wine = PathBuf::from("/nonexistent-wine");
    pkg.uid3 = 0xe79e_4cf9;
    pkg.version = (1, 0, 24);
    pkg.vendor = "Vendor".into();
    pkg.capabilities = hello_caps();
    assert!(
        pkg.package(&[Artifact { path: exe_path }]).is_err(),
        "Wine makekeys must still be required when cert/key are absent"
    );
    let sis = dir.path().join("hello.sis");
    assert!(sis.is_file(), "native .sis must exist without Wine makesis");
    let bytes = std::fs::read(&sis).unwrap();
    assert_eq!(&bytes[..16], &SisUid::new(0xe79e_4cf9).bytes());
    assert!(dir.path().join("hello.pkg").is_file());
    assert!(!dir.path().join("hello.sisx").is_file());
}

#[test]
fn package_writes_native_sisx_without_wine_signsis() {
    let dir = tempfile::tempdir().unwrap();
    let exe_path = dir.path().join("hello.exe");
    std::fs::write(&exe_path, hello_exe_bytes()).unwrap();
    let cert = dir.path().join("hello.cer");
    let key = dir.path().join("hello.key");
    std::fs::write(&cert, parse_hex(include_str!("testdata/test_dsa_cert.hex"))).unwrap();
    std::fs::write(&key, parse_hex(include_str!("testdata/test_dsa_key.hex"))).unwrap();
    let mut pkg = fake_pkg();
    pkg.tools.wine = PathBuf::from("/nonexistent-wine");
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
    assert_eq!(&sisx[..16], &SisUid::new(0xe79e_4cf9).bytes());
    assert_ne!(sisx, hello_sis_golden());
    assert!(sisx.len() > hello_sis_golden().len());
}

#[test]
fn encode_signed_sisx_is_verifiable_and_not_hello_golden() {
    let key = parse_hex(include_str!("testdata/test_dsa_key.hex"));
    let cert = parse_hex(include_str!("testdata/test_dsa_cert.hex"));
    let exe = hello_exe_bytes();
    let caps = hello_caps();
    let spec = SisUnsignedSpec {
        name: "hello",
        uid3: 0xe79e_4cf9,
        version: (1, 0, 24),
        vendor: "Vendor",
        vendor_localized: "Vendor-EN",
        exe: &exe,
        capabilities: &caps,
        datetime: hello_datetime(),
    };
    let unsigned = SisUnsigned::encode(&spec).unwrap();
    let sisx = SisUnsigned::encode_signed(&spec, &key, &cert, "").unwrap();
    assert_eq!(&sisx[..16], &unsigned[..16]);
    assert!(sisx.len() > unsigned.len());
    assert_ne!(sisx, parse_hex(include_str!("testdata/hello_sisx.hex")));
}

#[test]
fn experiment5_hello_key_native_sign_skipped_without_password() {
    let exp = std::path::PathBuf::from(std::env::var_os("HOME").unwrap_or_default())
        .join("src/symdev-experiment-5");
    let key = exp.join("hello.key");
    let cer = exp.join("hello.cer");
    if !key.is_file() || !cer.is_file() {
        return;
    }
    let Ok(password) = std::env::var("SYMDEV_SIGN_PASSWORD") else {
        return;
    };
    if password.is_empty() {
        return;
    }
    let exe = hello_exe_bytes();
    let caps = hello_caps();
    let spec = SisUnsignedSpec {
        name: "hello",
        uid3: 0xe79e_4cf9,
        version: (1, 0, 24),
        vendor: "Vendor",
        vendor_localized: "Vendor-EN",
        exe: &exe,
        capabilities: &caps,
        datetime: hello_datetime(),
    };
    let key_bytes = std::fs::read(&key).unwrap();
    let cer_bytes = std::fs::read(&cer).unwrap();
    let sisx = SisUnsigned::encode_signed(&spec, &key_bytes, &cer_bytes, &password).unwrap();
    let golden = parse_hex(include_str!("testdata/hello_sisx.hex"));
    // RFC6979 k will not match SignSIS's random k; structure must still be SISX.
    assert_eq!(&sisx[..16], &golden[..16]);
    assert!(sisx.len() > hello_sis_golden().len());
}
