//! `Caption`: one language's name for the application, as the launcher shows it.
use symdev_locale::{Language, Locales};
use symdev_manifest::UiApp;

/// The caption pair in one language, completed from the manifest where the locales
/// file translates only one of the two.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Caption {
    pub language: Language,
    pub caption: String,
    pub short_caption: String,
}

impl Caption {
    /// Every language whose locales file translates the caption or the short caption.
    ///
    /// A language that translates neither gets no file of its own: the launcher's
    /// `NearestLanguageFile` then falls back to `<app>.rsc`, which carries the
    /// manifest's pair — exactly what that language would have got anyway.
    pub fn of(ui: &UiApp, locales: Option<&Locales>) -> Vec<Caption> {
        let Some(locales) = locales else {
            return Vec::new();
        };
        locales
            .variants
            .iter()
            .filter(|(_, t)| t.caption().is_some() || t.short_caption().is_some())
            .map(|(language, table)| Caption {
                language: *language,
                caption: table.caption().unwrap_or(&ui.caption).to_string(),
                short_caption: table
                    .short_caption()
                    .unwrap_or(&ui.short_caption)
                    .to_string(),
            })
            .collect()
    }
}
