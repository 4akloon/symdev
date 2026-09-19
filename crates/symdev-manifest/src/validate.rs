use std::path::PathBuf;

use crate::error::{Error, Result};
use crate::schema::{
    Compiler, Manifest, Package, Platform, RawManifest, Signing, SigningMode, Symbian, Target,
    Toolchain,
};

pub const USER_GRANTABLE: &[&str] = &[
    "LocalServices",
    "NetworkServices",
    "ReadUserData",
    "WriteUserData",
    "UserEnvironment",
    "Location",
];

const PRIVILEGED: &[&str] = &[
    "PowerMgmt",
    "ProtServ",
    "ReadDeviceData",
    "SurroundingsDD",
    "SwEvent",
    "TrustedUI",
    "WriteDeviceData",
];

pub(crate) fn validate(raw: RawManifest) -> Result<Manifest> {
    Ok(Manifest {
        package: package(raw.package.name, raw.package.version)?,
        target: Target {
            device: raw.target.device,
        },
        platform: platform(raw.platform)?,
        language: raw.language.name,
        toolchain: toolchain(raw.toolchain)?,
        symbian: symbian(raw.symbian)?,
        signing: signing(raw.signing)?,
    })
}

fn package(name: String, version: String) -> Result<Package> {
    if !valid_name(&name) {
        return Err(Error::Invalid(format!("invalid package.name `{name}`")));
    }
    Ok(Package {
        name,
        version: parse_version(&version)?,
    })
}

fn valid_name(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() => chars.all(|c| c.is_ascii_alphanumeric() || c == '_'),
        _ => false,
    }
}

fn parse_version(s: &str) -> Result<(u32, u32, u32)> {
    let mut parts = s.split('.');
    let Some(major) = parts.next() else {
        return Err(bad_version(s));
    };
    let Some(minor) = parts.next() else {
        return Err(bad_version(s));
    };
    let Some(patch) = parts.next() else {
        return Err(bad_version(s));
    };
    if parts.next().is_some() {
        return Err(bad_version(s));
    }
    match (parse_u32(major), parse_u32(minor), parse_u32(patch)) {
        (Some(major), Some(minor), Some(patch)) => Ok((major, minor, patch)),
        _ => Err(bad_version(s)),
    }
}

fn parse_u32(s: &str) -> Option<u32> {
    if s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    s.parse().ok()
}

fn bad_version(s: &str) -> Error {
    Error::Invalid(format!(
        "package.version `{s}` must be three non-negative integers"
    ))
}

fn platform(raw: Option<crate::schema::RawPlatform>) -> Result<Platform> {
    match raw {
        None => Ok(inferred_platform()),
        Some(p) => {
            if p.family != "s60" || p.version != "3rd" || p.feature_pack != "fp2" {
                return Err(Error::Invalid(
                    "platform must be family=s60, version=3rd, feature_pack=fp2".into(),
                ));
            }
            Ok(Platform {
                family: p.family,
                version: p.version,
                feature_pack: p.feature_pack,
            })
        }
    }
}

fn inferred_platform() -> Platform {
    Platform {
        family: "s60".into(),
        version: "3rd".into(),
        feature_pack: "fp2".into(),
    }
}

fn toolchain(raw: Option<crate::schema::RawToolchain>) -> Result<Toolchain> {
    let Some(raw) = raw else {
        return Ok(Toolchain {
            sdk: None,
            compiler: Compiler::Gcce14,
        });
    };
    if let Some(sdk) = raw.sdk.as_deref()
        && sdk != "s60-3rd-fp2"
    {
        return Err(Error::Invalid(format!(
            "toolchain.sdk `{sdk}` must be s60-3rd-fp2"
        )));
    }
    Ok(Toolchain {
        sdk: raw.sdk,
        compiler: raw.compiler.unwrap_or(Compiler::Gcce14),
    })
}

fn symbian(raw: Option<crate::schema::RawSymbian>) -> Result<Symbian> {
    let Some(raw) = raw else {
        return Ok(Symbian {
            uid3: None,
            capabilities: Vec::new(),
            vendor: "symdev".into(),
            icon: None,
        });
    };
    Ok(Symbian {
        uid3: match raw.uid3 {
            Some(s) => Some(parse_uid3(&s)?),
            None => None,
        },
        capabilities: capabilities(raw.capabilities)?,
        vendor: vendor(raw.vendor)?,
        icon: nonempty_path(raw.icon, "symbian.icon")?,
    })
}

fn parse_uid3(s: &str) -> Result<u32> {
    let rest = s.as_bytes();
    if rest.len() != 10 || !s.starts_with("0x") || !s[2..].bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(Error::Invalid(format!(
            "symbian.uid3 `{s}` must match 0x + 8 hex digits"
        )));
    }
    let uid = u32::from_str_radix(&s[2..], 16)
        .map_err(|_| Error::Invalid(format!("symbian.uid3 `{s}` must match 0x + 8 hex digits")))?;
    let in_a = (0xA000_0000..=0xAFFF_FFFF).contains(&uid);
    let in_e = (0xE000_0000..=0xEFFF_FFFF).contains(&uid);
    if !(in_a || in_e) {
        return Err(Error::Invalid(format!(
            "symbian.uid3 `{s}` is outside the self-sign ranges"
        )));
    }
    Ok(uid)
}

fn capabilities(caps: Vec<String>) -> Result<Vec<String>> {
    let mut out = Vec::with_capacity(caps.len());
    for cap in caps {
        if !USER_GRANTABLE.contains(&cap.as_str()) {
            let kind = if PRIVILEGED.contains(&cap.as_str()) {
                "privileged"
            } else {
                "unknown"
            };
            return Err(Error::Invalid(format!("{kind} capability `{cap}`")));
        }
        if out.iter().any(|seen| seen == &cap) {
            return Err(Error::Invalid(format!("duplicate capability `{cap}`")));
        }
        out.push(cap);
    }
    Ok(out)
}

fn vendor(vendor: Option<String>) -> Result<String> {
    match vendor {
        None => Ok("symdev".into()),
        Some(v) if v.is_empty() => Err(Error::Invalid("symbian.vendor must be non-empty".into())),
        Some(v) => Ok(v),
    }
}

fn signing(raw: Option<crate::schema::RawSigning>) -> Result<Signing> {
    let Some(raw) = raw else {
        return Ok(Signing {
            mode: SigningMode::SelfSigned,
            cert: None,
            key: None,
            subject: None,
        });
    };
    Ok(Signing {
        mode: raw.mode.unwrap_or(SigningMode::SelfSigned),
        cert: nonempty_path(raw.cert, "signing.cert")?,
        key: nonempty_path(raw.key, "signing.key")?,
        subject: match raw.subject {
            Some(s) if s.trim().is_empty() => {
                return Err(Error::Invalid("signing.subject must not be empty".into()));
            }
            other => other,
        },
    })
}

fn nonempty_path(path: Option<PathBuf>, field: &str) -> Result<Option<PathBuf>> {
    match path {
        None => Ok(None),
        Some(p) if p.as_os_str().is_empty() => {
            Err(Error::Invalid(format!("{field} must be a non-empty path")))
        }
        Some(p) => Ok(Some(p)),
    }
}
