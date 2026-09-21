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
//! short_caption = "Notes"      # optional; defaults to `caption`
//! softkeys = "options-exit"    # optional; "options-exit" when there is a menu
//! left_softkey = "Options"     # optional; the label only, never the command
//! right_softkey = "Exit"       # optional
//!
//! [[ui.menu]]
//! id = "new"                   # the word the Rust source matches with Command::named
//! label = "New note"
//! ```
use serde::Deserialize;

use crate::command_id::CommandId;
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
    /// The Options menu, in the order it is shown. Empty means no menu bar.
    pub menu: Vec<MenuItem>,
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum Softkeys {
    #[serde(rename = "exit")]
    Exit,
    #[serde(rename = "options-exit")]
    OptionsExit,
}

/// One line of the Options menu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuItem {
    /// The name the Rust source repeats in `Command::named`.
    pub name: String,
    /// What the person reads.
    pub label: String,
    /// What `HandleCommandL` is handed, derived from [`MenuItem::name`].
    pub command: CommandId,
}

impl MenuItem {
    /// The Rust constant `#[symbian_std::main(gui)]` writes for this item: the id in
    /// upper case with `-` as `_`, so `new-note` is matched as `menu::NEW_NOTE`.
    pub fn constant(&self) -> String {
        self.name.to_ascii_uppercase().replace('-', "_")
    }
}

impl UiApp {
    pub(crate) fn validate(raw: Option<RawUi>, package: &str) -> Result<Option<Self>> {
        let Some(raw) = raw else {
            return Ok(None);
        };
        let caption = text(raw.caption, "ui.caption")?.unwrap_or_else(|| package.to_string());
        let short_caption = text(raw.short_caption, "ui.short_caption")?.unwrap_or(caption.clone());
        let menu = menu(raw.menu)?;
        // A menu with no way to open it is dead weight, and an Options softkey with no
        // menu bar is worse than dead: the framework reaches for the menu bar the
        // moment the key is pressed and the application dies with an access violation
        // (observed in EKA2L1, experiment 91). So the default follows the menu, and
        // the contradiction is refused rather than shipped.
        let softkeys = raw.softkeys.unwrap_or(if menu.is_empty() {
            Softkeys::Exit
        } else {
            Softkeys::OptionsExit
        });
        if softkeys == Softkeys::OptionsExit && menu.is_empty() {
            return Err(Error::Invalid(
                "ui.softkeys = \"options-exit\" needs at least one [[ui.menu]] item: \
                 the left softkey opens the menu bar, and there would be none"
                    .into(),
            ));
        }
        Ok(Some(Self {
            kind: raw.kind,
            caption,
            short_caption,
            softkeys,
            left_softkey: text(raw.left_softkey, "ui.left_softkey")?
                .unwrap_or_else(|| "Options".into()),
            right_softkey: text(raw.right_softkey, "ui.right_softkey")?
                .unwrap_or_else(|| "Exit".into()),
            menu,
        }))
    }
}

/// The menu, with the two things that make one useless caught here rather than in the
/// emulator: two items a person cannot tell apart, and two names that happen to hash
/// to one command, which would silently join two lines into one.
fn menu(raw: Option<Vec<RawMenuItem>>) -> Result<Vec<MenuItem>> {
    let mut items: Vec<MenuItem> = Vec::new();
    for entry in raw.unwrap_or_default() {
        let name = required(entry.id, "ui.menu.id")?;
        let label = required(entry.label, "ui.menu.label")?;
        let command = CommandId::of(&name);
        identifier(&name)?;
        if let Some(clash) = items.iter().find(|i| i.name == name) {
            return Err(Error::Invalid(format!(
                "ui.menu has two items named {:?} ({:?} and {:?})",
                name, clash.label, label
            )));
        }
        let constant = name.to_ascii_uppercase().replace('-', "_");
        if let Some(clash) = items.iter().find(|i| i.constant() == constant) {
            return Err(Error::Invalid(format!(
                "ui.menu ids {:?} and {:?} would both be the constant `menu::{constant}`; rename one",
                clash.name, name
            )));
        }
        if let Some(clash) = items.iter().find(|i| i.command == command) {
            return Err(Error::Invalid(format!(
                "ui.menu ids {:?} and {:?} both become command {command}; rename one",
                clash.name, name
            )));
        }
        items.push(MenuItem {
            name,
            label,
            command,
        });
    }
    Ok(items)
}

/// An id becomes a Rust constant (`menu::NEW_NOTE` for `new-note`), so it is a lower
/// case ASCII word: a letter, then letters, digits, `-` or `_`. Anything else would be
/// a constant the source cannot spell.
fn identifier(name: &str) -> Result<()> {
    let mut chars = name.chars();
    let well_formed = chars.next().is_some_and(|c| c.is_ascii_lowercase())
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_');
    if well_formed {
        Ok(())
    } else {
        Err(Error::Invalid(format!(
            "ui.menu.id {name:?} must be a lower case ASCII word (a letter, then letters, \
             digits, `-` or `_`): it becomes the Rust constant `menu::{}`",
            name.to_ascii_uppercase().replace('-', "_")
        )))
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

fn required(value: Option<String>, field: &str) -> Result<String> {
    text(value, field)?.ok_or_else(|| Error::Invalid(format!("{field} is required")))
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
    pub(crate) menu: Option<Vec<RawMenuItem>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawMenuItem {
    pub(crate) id: Option<String>,
    pub(crate) label: Option<String>,
}

#[cfg(test)]
mod tests;
