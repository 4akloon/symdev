mod error;
mod icons;
mod install;
mod schema;
mod ui;
mod validate;

pub use error::{Error, Result};
pub use icons::{IconContainer, IconSource};
pub use install::InstallFile;
pub use schema::{
    Compiler, Device, Language, Manifest, Package, Platform, Signing, SigningMode, Symbian, Target,
    Toolchain,
};
pub use ui::{Softkeys, UiApp, UiKind};
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
mod tests;
