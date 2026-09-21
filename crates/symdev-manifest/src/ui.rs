//! `[ui]`: the section that makes a project a GUI application.
//!
//! Its presence is the switch. A manifest without it builds a console program that
//! links no Avkon subclass, gets the empty fallback registration resource and has no
//! caption; a manifest with it gets the C++ shim, the six Avkon libraries and the
//! `.rsc` / `_reg.rsc` / `.mif` a captioned application needs
//! (`docs/research/avkon-rust-spec.md` §6).
//!
//! What belongs here and what does not follows one rule: **the manifest holds what the
//! phone needs before the application runs.** The caption and the icon are read by the
//! launcher without launching anything, and the softkey pair is a compiled resource
//! the framework reads as the application starts. The Options menu is not here at all
//! — it is only ever needed while the application runs, so it is declared in Rust
//! (`symbian_ui::App::menu`) and added to an empty pane at the moment it opens
//! (experiment 95).
//!
//! ```toml
//! [ui]
//! kind = "avkon"
//! caption = "Notes"
//! short_caption = "Notes"      # optional; defaults to `caption`
//! softkeys = "options-exit"    # optional; "exit" unless the application has a menu
//! left_softkey = "Options"     # optional; the label only, never the command
//! right_softkey = "Exit"       # optional
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
    /// The label on the left softkey, which is also the menu title.
    pub left_softkey: String,
    pub right_softkey: String,
}

/// Which application framework. There is one, and naming it is what leaves room for a
/// second without the section changing meaning.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum UiKind {
    /// `CAknApplication` / `CAknDocument` / `CAknAppUi` / `CCoeControl`, S60 3rd FP2.
    #[serde(rename = "avkon")]
    Avkon,
}

/// Which pair of buttons the generated `CBA` carries.
///
/// The button *group* is generated into the application's own `.rss` rather than named
/// from Avkon's, so that the commands it sends are ones symdev chose — see
/// `UiResources::app_rss`. What these two values choose is the command on the **left**
/// button, and only that: `Exit` leaves it empty, `OptionsExit` makes it
/// `EAknSoftkeyOptions`, which is the command the framework itself watches for in
/// order to open the menu bar.
///
/// So this is also where an application says it *has* an Options menu, and it has to
/// be said here rather than in Rust: the button group and the menu bar are both
/// compiled resources, written before any Rust runs. `OptionsExit` with no menu bar is
/// an access violation the instant the key is pressed (experiment 91), which is why
/// `UiResources` emits the two **together** — the contradiction this section used to
/// refuse can no longer be written down (experiment 95).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum Softkeys {
    #[serde(rename = "exit")]
    Exit,
    #[serde(rename = "options-exit")]
    OptionsExit,
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
            // There is no menu here to follow any more, so the left button is empty
            // unless the application says it has one to open.
            softkeys: raw.softkeys.unwrap_or(Softkeys::Exit),
            left_softkey: text(raw.left_softkey, "ui.left_softkey")?
                .unwrap_or_else(|| "Options".into()),
            right_softkey: text(raw.right_softkey, "ui.right_softkey")?
                .unwrap_or_else(|| "Exit".into()),
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
    pub(crate) left_softkey: Option<String>,
    pub(crate) right_softkey: Option<String>,
}

#[cfg(test)]
mod tests;
