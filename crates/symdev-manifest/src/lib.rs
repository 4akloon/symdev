mod error;
mod schema;
mod validate;

pub use error::{Error, Result};
pub use schema::{
    Compiler, Device, Language, Manifest, Package, Platform, Signing, SigningMode, Symbian, Target,
    Toolchain,
};
pub use validate::USER_GRANTABLE;

use std::path::Path;

pub fn parse(src: &str) -> Result<Manifest> {
    let raw: schema::RawManifest =
        toml::from_str(src).map_err(|e| Error::Invalid(e.to_string()))?;
    validate::validate(raw)
}

pub fn load(path: impl AsRef<Path>) -> Result<Manifest> {
    match std::fs::read_to_string(path) {
        Ok(src) => parse(&src),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Err(Error::MissingFile),
        Err(e) => Err(Error::Invalid(e.to_string())),
    }
}

#[cfg(test)]
extern crate self as symdev_manifest;

#[cfg(test)]
const HELLO: &str = r#"
[package]
name = "hello"
version = "0.1.0"

[target]
device = "nokia-e52"

[language]
name = "cpp"

[symbian]
capabilities = []
vendor = "symdev"

[signing]
mode = "self-signed"
"#;

#[cfg(test)]
fn reject(src: &str) {
    let err = symdev_manifest::parse(src).unwrap_err();
    let msg = err.to_string();
    assert!(!msg.is_empty(), "error reason must be non-empty");
    assert!(
        !msg.starts_with("invalid manifest:"),
        "crate Error is the reason after CLI prefix, got {msg}"
    );
}

#[test]
fn parse_canonical_hello() {
    let m = symdev_manifest::parse(HELLO).unwrap();
    assert_eq!(m.package.name, "hello");
    assert_eq!(m.package.version, (0, 1, 0));
    assert_eq!(m.target.device, symdev_manifest::Device::NokiaE52);
    assert_eq!(m.platform.family, "s60");
    assert_eq!(m.platform.version, "3rd");
    assert_eq!(m.platform.feature_pack, "fp2");
    assert_eq!(m.language, symdev_manifest::Language::Cpp);
    assert_eq!(m.toolchain.compiler, symdev_manifest::Compiler::Gcce14);
    assert_eq!(m.toolchain.sdk, None);
    assert_eq!(m.symbian.uid3, None);
    assert!(m.symbian.capabilities.is_empty());
    assert_eq!(m.symbian.vendor, "symdev");
    assert_eq!(m.signing.mode, symdev_manifest::SigningMode::SelfSigned);
    assert_eq!(m.signing.cert, None);
    assert_eq!(m.signing.key, None);
}

#[test]
fn parse_explicit_uid3_and_user_grantable_cap() {
    let src = r#"
[package]
name = "hello"
version = "0.1.0"

[target]
device = "nokia-e52"

[language]
name = "cpp"

[symbian]
uid3 = "0xA0000001"
capabilities = ["NetworkServices"]

[signing]
mode = "self-signed"
"#;
    let m = symdev_manifest::parse(src).unwrap();
    assert_eq!(m.symbian.uid3, Some(0xA000_0001));
    assert_eq!(m.symbian.capabilities, ["NetworkServices"]);
    assert_eq!(m.symbian.vendor, "symdev");
}

#[test]
fn parse_explicit_uid3_test_range() {
    let m = symdev_manifest::parse(&HELLO.replace(
        "capabilities = []",
        "uid3 = \"0xE0000001\"\ncapabilities = []",
    ))
    .unwrap();
    assert_eq!(m.symbian.uid3, Some(0xE000_0001));
}

#[test]
fn omitted_platform_infers_s60_3rd_fp2() {
    assert!(
        !HELLO.contains("[platform]"),
        "canonical hello must omit [platform]"
    );
    let m = symdev_manifest::parse(HELLO).unwrap();
    assert_eq!(m.platform.family, "s60");
    assert_eq!(m.platform.version, "3rd");
    assert_eq!(m.platform.feature_pack, "fp2");
}

#[test]
fn omitted_toolchain_defaults_gcce14() {
    assert!(
        !HELLO.contains("[toolchain]"),
        "canonical hello must omit [toolchain]"
    );
    let m = symdev_manifest::parse(HELLO).unwrap();
    assert_eq!(m.toolchain.compiler, symdev_manifest::Compiler::Gcce14);
}

#[test]
fn reject_unknown_device() {
    reject(&HELLO.replace("nokia-e52", "nokia-n95"));
}

#[test]
fn reject_java_language() {
    reject(&HELLO.replace(r#"name = "cpp""#, r#"name = "java""#));
}

#[test]
fn reject_protected_uid3() {
    reject(&HELLO.replace(
        "capabilities = []",
        "uid3 = \"0x1000007a\"\ncapabilities = []",
    ));
}

#[test]
fn reject_privileged_capability() {
    reject(&HELLO.replace("capabilities = []", r#"capabilities = ["PowerMgmt"]"#));
}

#[test]
fn reject_unknown_capability() {
    reject(&HELLO.replace("capabilities = []", r#"capabilities = ["Foo"]"#));
}

#[test]
fn reject_duplicate_capability() {
    reject(&HELLO.replace(
        "capabilities = []",
        r#"capabilities = ["NetworkServices", "NetworkServices"]"#,
    ));
}

#[test]
fn reject_unknown_table() {
    reject(&format!("{HELLO}\n[widgets]\nx = 1\n"));
}

#[test]
fn reject_signing_devcert() {
    reject(&HELLO.replace("self-signed", "devcert"));
}

#[test]
fn reject_bad_package_name() {
    reject(&HELLO.replace(r#"name = "hello""#, r#"name = "1bad""#));
}

#[test]
fn reject_two_part_version() {
    reject(&HELLO.replace(r#"version = "0.1.0""#, r#"version = "1.0""#));
}

#[test]
fn reject_partial_platform() {
    reject(&format!("{HELLO}\n[platform]\nfamily = \"s60\"\n"));
}

#[test]
fn load_missing_file() {
    let err = symdev_manifest::load("/no/such/dir/symdev.toml").unwrap_err();
    assert!(matches!(err, symdev_manifest::Error::MissingFile));
    assert_eq!(err.to_string(), "no symdev.toml in current directory");
}
