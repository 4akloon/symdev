//! Per-language locales files: the one reader both the build and the compile-time
//! string constants use, so the two can never disagree about what a key is or which
//! resource index it lands at.
//!
//! `locales/default.toml` is the fallback — compiled into `<app>_strings.rsc`, which
//! `BaflUtils::NearestLanguageFile` leaves in place when no variant matches the device.
//! `locales/<language>.toml` is a variant, compiled into `<app>_strings.r<code>`. See
//! `docs/superpowers/plans/2026-09-22-native-localisation.md` for the whole contract.
mod file;
mod language;
mod locales;

pub use file::Table;
pub use language::Language;
pub use locales::Locales;

/// What went wrong reading a locales directory, naming the file and line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error(pub String);

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Error {}

#[cfg(test)]
mod tests;
