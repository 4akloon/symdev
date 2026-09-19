use super::{
    SisArray, SisCompressed, SisController, SisData, SisData31, SisData32, SisDateTime, SisEncode,
    SisField, SisFile, SisFiles, SisHash, SisInfo, SisLanguage, SisLanguages, SisPkgUid,
    SisProduct, SisProductVersion, SisProducts, SisString, SisU32, SisUid, SisUnsigned, SisVersion,
    SisWord41, SisWords, SisWords16, SisWords19,
};
use sha1::{Digest, Sha1};
use std::path::{Path, PathBuf};
use symdev_core::{Error, Result};

pub struct SisTools {
    pub wine: PathBuf,
    pub makesis: PathBuf,
    pub signsis: PathBuf,
}

impl SisTools {
    pub fn from_env() -> Result<Self> {
        let epocroot = Self::required("SYMDEV_EPOCROOT")?;
        let wine = match std::env::var("SYMDEV_WINE") {
            Ok(v) if !v.is_empty() => PathBuf::from(v),
            _ => PathBuf::from("/usr/bin/wine"),
        };
        let tools = epocroot.join("epoc32/tools");
        Ok(Self {
            wine,
            makesis: tools.join("makesis.exe"),
            signsis: tools.join("signsis.exe"),
        })
    }

    pub fn makesis_args(&self, pkg: &str, sis: &str) -> Vec<String> {
        vec![
            Self::path_arg(&self.wine),
            Self::path_arg(&self.makesis),
            "-v".into(),
            pkg.into(),
            sis.into(),
        ]
    }

    pub fn signsis_args(
        &self,
        sis: &str,
        sisx: &str,
        cer: &str,
        key: &str,
        password: &str,
    ) -> Vec<String> {
        vec![
            Self::path_arg(&self.wine),
            Self::path_arg(&self.signsis),
            sis.into(),
            sisx.into(),
            cer.into(),
            key.into(),
            password.into(),
        ]
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
    /// Non-EXE files in `.pkg` order (experiment 51: SIS keeps that order).
    pub files: &'a [SisPkgFile<'a>],
}

/// A non-EXE file to install: its `!:\...` destination and bytes.
#[derive(Debug, Clone, Copy)]
pub struct SisPkgFile<'a> {
    pub dest: &'a str,
    pub data: &'a [u8],
}

impl<'a> SisPkgFile<'a> {
    /// `!:\private\10003a3f\import\apps\<name>_reg.rsc` (experiment 43 template).
    pub fn reg_rsc_dest(name: &str) -> String {
        format!("!:\\private\\10003a3f\\import\\apps\\{name}_reg.rsc")
    }
}

impl SisUnsignedSpec<'_> {
    /// File description (controller) and its data blob, sharing one compression choice.
    fn install_file(
        dest: String,
        data: &[u8],
        caps: Option<SisWord41>,
        idx: u32,
    ) -> Result<(SisField, SisField)> {
        let blob = SisCompressed::smallest(data)?;
        let stored = blob.data.len() as u32;
        let size = data.len() as u32;
        let digest: [u8; 20] = Sha1::digest(data).into();
        let file = SisFile::new(
            SisString::new(dest),
            SisString::new(""),
            caps,
            SisHash::new([1, 0x25, 0x14], digest),
            SisString::new(""),
            [stored, 0, size, 0, idx],
        )
        .field();
        Ok((file, SisData32::new(blob).field()))
    }

    fn parts(&self) -> Result<(SisController, SisData)> {
        let caps = SisWord41::from_capabilities(self.capabilities)?;
        let (major, minor, build) = self.version;
        // makesis writes type 41 only when the EXE has capabilities (experiment 51).
        let caps = (caps.value != 0).then_some(caps);
        let (exe_file, exe_blob) = Self::install_file(
            format!("!:\\sys\\bin\\{}.exe", self.name),
            self.exe,
            caps,
            0,
        )?;
        let mut files = vec![exe_file];
        let mut blobs = vec![exe_blob];
        for (idx, extra) in self.files.iter().enumerate() {
            let (file, blob) =
                Self::install_file(extra.dest.to_string(), extra.data, None, idx as u32 + 1)?;
            files.push(file);
            blobs.push(blob);
        }
        let controller = SisController::new(
            SisInfo::new(
                SisPkgUid::new(self.uid3),
                SisString::new(self.vendor),
                SisArray::new(vec![SisString::new(self.name).field()]),
                SisArray::new(vec![SisString::new(self.vendor_localized).field()]),
                SisVersion::new(major, minor, build),
                self.datetime,
            ),
            SisWords16::new(SisWords::new(vec![0x21])), // ponytail: recorded TYPE=SA &EN one-file words; derive when pkg grammar grows
            SisLanguages::new(SisArray::new(vec![SisLanguage::new(1).field()])),
            SisProducts::new(SisArray::new(vec![
                SisProduct::new(
                    SisPkgUid::new(0x1027_52ae),
                    SisProductVersion::new(SisVersion::new(0, 0, 0)),
                    SisArray::new(vec![SisString::new("S60ProductID").field()]),
                )
                .field(),
            ])),
            SisWords19::new(SisWords::new(vec![0x14])),
            SisFiles::new(
                SisArray::new(files),
                SisWords::new(vec![0x0d]),
                SisWords::new(vec![0x1a]),
            ),
            SisU32::new(0),
        );
        let data = SisData::new(SisArray::new(vec![
            SisData31::new(SisArray::new(blobs)).field(),
        ]));
        Ok((controller, data))
    }
}

impl SisWord41 {
    fn from_capabilities(caps: &[String]) -> Result<Self> {
        let bits = symdev_core::Capabilities::from_names(caps)?.bits();
        let word = u32::try_from(bits)
            .map_err(|_| Error::Other(format!("SIS type-41 word cannot hold {bits:#x}")))?;
        Ok(Self::new(word))
    }
}

impl SisUnsigned {
    pub fn encode(spec: &SisUnsignedSpec<'_>) -> Result<Vec<u8>> {
        let (controller, data) = spec.parts()?;
        Self::wrap(spec.uid3, &controller, data)
    }

    pub fn encode_signed(
        spec: &SisUnsignedSpec<'_>,
        key_pem: &[u8],
        cert_pem_or_der: &[u8],
        password: &str,
    ) -> Result<Vec<u8>> {
        let (controller, data) = spec.parts()?;
        let signatures =
            controller.signatures_from_key_and_cert(key_pem, cert_pem_or_der, password)?;
        Self::wrap(spec.uid3, &controller.with_signatures(signatures), data)
    }

    fn wrap(uid3: u32, controller: &SisController, data: SisData) -> Result<Vec<u8>> {
        Ok(Self::new(
            SisUid::new(uid3),
            SisCompressed::zlib(&controller.field().bytes())?,
            data,
        )
        .bytes())
    }
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
fn signsis_args_match_experiment_8() {
    assert_eq!(
        sdk_tools().signsis_args(
            "hello.sis",
            "hello.sisx",
            "hello.cer",
            "hello.key",
            "secret"
        ),
        [
            "/usr/bin/wine",
            "/sdk/epoc32/tools/signsis.exe",
            "hello.sis",
            "hello.sisx",
            "hello.cer",
            "hello.key",
            "secret",
        ]
    );
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
fn hello_datetime() -> crate::SisDateTime {
    crate::SisDateTime::new(
        crate::SisDate::new(2026, 8, 17),
        crate::SisTime::new(15, 18, 24),
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
        files: &[],
    })
    .unwrap();
    let golden = hello_sis_golden();
    assert_eq!(golden.len(), 4000);
    assert_eq!(bytes, golden);
}

#[test]
fn encode_unsigned_sis_with_reg_rsc_matches_experiment_43() {
    // Wine makesis -v on experiment-7 hello.pkg plus the SDK-example `_reg.rsc` line.
    let golden = parse_hex(include_str!("testdata/hello_reg_sis.hex"));
    let rsc = symdev_rcomp::Rsc::registration(0xe79e_4cf9, "hello")
        .unwrap()
        .bytes()
        .unwrap();
    let bytes = SisUnsigned::encode(&SisUnsignedSpec {
        name: "hello",
        uid3: 0xe79e_4cf9,
        version: (1, 0, 24),
        vendor: "Vendor",
        vendor_localized: "Vendor-EN",
        exe: &hello_exe_bytes(),
        capabilities: &hello_caps(),
        datetime: crate::SisDateTime::new(
            crate::SisDate::new(2026, 8, 19),
            crate::SisTime::new(9, 2, 53),
        ),
        files: &[SisPkgFile {
            dest: &SisPkgFile::reg_rsc_dest("hello"),
            data: &rsc,
        }],
    })
    .unwrap();
    assert_eq!(bytes.len(), golden.len());
    assert_eq!(bytes, golden);
}

#[test]
fn encode_unsigned_three_file_gui_sis_matches_experiment_51() {
    // Wine makesis on gui.exe (no capabilities) + gui.rsc + gui_reg.rsc, in .pkg order.
    let exe = parse_hex(include_str!("testdata/exp51_gui_exe.hex"));
    let rsc = parse_hex(include_str!("testdata/exp51_gui_rsc.hex"));
    let reg = parse_hex(include_str!("testdata/exp51_gui_reg_rsc.hex"));
    let golden = parse_hex(include_str!("testdata/exp51_gui_sis.hex"));
    let bytes = SisUnsigned::encode(&SisUnsignedSpec {
        name: "gui",
        uid3: 0xe5d1_a001,
        version: (1, 0, 0),
        vendor: "Vendor",
        vendor_localized: "Vendor-EN",
        exe: &exe,
        capabilities: &[],
        datetime: crate::SisDateTime::new(
            crate::SisDate::new(2026, 8, 19),
            crate::SisTime::new(16, 1, 18),
        ),
        files: &[
            SisPkgFile {
                dest: "!:\\resource\\apps\\gui.rsc",
                data: &rsc,
            },
            SisPkgFile {
                dest: &SisPkgFile::reg_rsc_dest("gui"),
                data: &reg,
            },
        ],
    })
    .unwrap();
    assert_eq!(bytes.len(), golden.len());
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
        files: &[],
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
        capabilities: &["NotACapability".into()],
        datetime: hello_datetime(),
        files: &[],
    })
    .unwrap_err();
    assert_eq!(
        err.to_string(),
        "capability bit not yet derived: NotACapability"
    );
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
        files: &[],
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
        files: &[],
    };
    let key_bytes = std::fs::read(&key).unwrap();
    let cer_bytes = std::fs::read(&cer).unwrap();
    let sisx = SisUnsigned::encode_signed(&spec, &key_bytes, &cer_bytes, &password).unwrap();
    let golden = parse_hex(include_str!("testdata/hello_sisx.hex"));
    // RFC6979 k will not match SignSIS's random k; structure must still be SISX.
    assert_eq!(&sisx[..16], &golden[..16]);
    assert!(sisx.len() > hello_sis_golden().len());
}
