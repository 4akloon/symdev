use std::path::PathBuf;

use serde::Deserialize;

use crate::icons::IconContainer;
use crate::install::InstallFile;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Manifest {
    pub package: Package,
    pub target: Target,
    pub platform: Platform,
    pub language: Language,
    pub toolchain: Toolchain,
    pub symbian: Symbian,
    pub signing: Signing,
    /// `[[install]]`: extra files the package carries.
    pub install: Vec<InstallFile>,
    /// `[[icons]]`: icon containers built by the native `mifconv`.
    pub icons: Vec<IconContainer>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Package {
    pub name: String,
    pub version: (u32, u32, u32),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    pub device: Device,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum Device {
    #[serde(rename = "nokia-e52")]
    NokiaE52,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Platform {
    pub family: String,
    pub version: String,
    pub feature_pack: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum Language {
    #[serde(rename = "cpp")]
    Cpp,
    /// A `no_std` Rust application built by `RustBuild` (experiment 65).
    #[serde(rename = "rust")]
    Rust,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Toolchain {
    pub sdk: Option<String>,
    pub compiler: Compiler,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum Compiler {
    #[serde(rename = "gcce-14")]
    Gcce14,
    #[serde(rename = "gcce-15")]
    Gcce15,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Symbian {
    pub uid3: Option<u32>,
    /// `secure_id`: the image's secure identity. Overrides an MMP `SECUREID`; when
    /// neither names one the post-linker defaults it to UID3.
    pub secure_id: Option<u32>,
    pub capabilities: Vec<String>,
    pub vendor: String,
    /// App icon SVG, relative to the project root (built into `<app>_aif.mif`).
    pub icon: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Signing {
    pub mode: SigningMode,
    pub cert: Option<PathBuf>,
    pub key: Option<PathBuf>,
    /// RFC 4514 subject for a generated self-signed cert; `None` = recorded example.
    pub subject: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum SigningMode {
    #[serde(rename = "self-signed")]
    SelfSigned,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawManifest {
    pub(crate) package: RawPackage,
    pub(crate) target: RawTarget,
    pub(crate) platform: Option<RawPlatform>,
    pub(crate) language: RawLanguage,
    pub(crate) toolchain: Option<RawToolchain>,
    pub(crate) symbian: Option<RawSymbian>,
    pub(crate) signing: Option<RawSigning>,
    #[serde(default)]
    pub(crate) install: Vec<crate::install::RawInstall>,
    #[serde(default)]
    pub(crate) icons: Vec<crate::icons::RawIconContainer>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawPackage {
    pub(crate) name: String,
    pub(crate) version: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawTarget {
    pub(crate) device: Device,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawPlatform {
    pub(crate) family: String,
    pub(crate) version: String,
    pub(crate) feature_pack: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawLanguage {
    pub(crate) name: Language,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawToolchain {
    pub(crate) sdk: Option<String>,
    pub(crate) compiler: Option<Compiler>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawSymbian {
    pub(crate) uid3: Option<String>,
    pub(crate) secure_id: Option<String>,
    #[serde(default)]
    pub(crate) capabilities: Vec<String>,
    pub(crate) vendor: Option<String>,
    pub(crate) icon: Option<PathBuf>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawSigning {
    pub(crate) mode: Option<SigningMode>,
    pub(crate) cert: Option<PathBuf>,
    pub(crate) key: Option<PathBuf>,
    pub(crate) subject: Option<String>,
}
