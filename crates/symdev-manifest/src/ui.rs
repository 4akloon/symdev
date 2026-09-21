//! `[ui]`: the section that makes a project a GUI application.
//!
//! Its presence is the switch. A manifest without it builds a console program that
//! links no Avkon subclass, gets the empty fallback registration resource and has no
//! caption; a manifest with it gets the C++ shim, the six Avkon libraries and the
//! `.rsc` / `_reg.rsc` / `.mif` a captioned application needs
//! (`docs/research/avkon-rust-spec.md` §6).
//!
//! ```toml
//! [ui]
//! kind = "avkon"
//! caption = "Notes"
//! short_caption = "Notes"   # optional; defaults to `caption`
//! softkeys = "exit"         # optional; defaults to "exit"
//! ```
use serde::Deserialize;

use crate::error::{Error, Result};

/// A GUI application, as `[ui]` declares it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiApp {
    pub kind: UiKind,
    /// `LOCALISABLE_APP_INFO`'s `caption`: the name under the icon.
    pub caption: String,
    /// `short_caption`, for the places S60 has less room. Defaults to `caption`.
    pub short_caption: String,
    pub softkeys: Softkeys,
}

/// Which application framework. There is one, and naming it is what leaves room for a
/// second without the section changing meaning.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum UiKind {
    /// `CAknApplication` / `CAknDocument` / `CAknAppUi` / `CCoeControl`, S60 3rd FP2.
    #[serde(rename = "avkon")]
    Avkon,
}

/// The button group the `.rss`'s `EIK_APP_INFO` names.
///
/// Only the one Avkon resource `examples/gui` is built against has been observed
/// working end to end, so it is the only value: `R_AVKON_SOFTKEYS_OPTIONS_EXIT` needs
/// a menu bar this step does not generate, and a guess is not acceptable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum Softkeys {
    #[serde(rename = "exit")]
    Exit,
}

impl Softkeys {
    /// The Avkon resource identifier `cba` is set to.
    pub fn resource(self) -> &'static str {
        match self {
            Self::Exit => "R_AVKON_SOFTKEYS_EXIT",
        }
    }
}

impl UiApp {
    pub(crate) fn validate(raw: Option<RawUi>, package: &str) -> Result<Option<Self>> {
        let Some(raw) = raw else {
            return Ok(None);
        };
        let caption = text(raw.caption, "ui.caption")?.unwrap_or_else(|| package.to_string());
        let short_caption = text(raw.short_caption, "ui.short_caption")?.unwrap_or(caption.clone());
        Ok(Some(Self {
            kind: raw.kind,
            caption,
            short_caption,
            softkeys: raw.softkeys.unwrap_or(Softkeys::Exit),
        }))
    }
}

/// A caption is shown to a person, so it may hold anything but nothing: an empty one
/// would leave the application nameless in the menu, which is the failure this section
/// exists to prevent.
fn text(value: Option<String>, field: &str) -> Result<Option<String>> {
    match value {
        None => Ok(None),
        Some(v) if v.trim().is_empty() => Err(Error::Invalid(format!("{field} must not be empty"))),
        Some(v) => Ok(Some(v)),
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawUi {
    pub(crate) kind: UiKind,
    pub(crate) caption: Option<String>,
    pub(crate) short_caption: Option<String>,
    pub(crate) softkeys: Option<Softkeys>,
}

#[cfg(test)]
mod tests;
