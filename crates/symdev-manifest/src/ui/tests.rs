//! What `[ui]` accepts, what it defaults and what it refuses.
use crate::tests::{HELLO, reject};
use crate::{Softkeys, UiKind, parse};

#[test]
fn ui_section_is_absent_for_a_console_project() {
    assert!(parse(HELLO).unwrap().ui.is_none());
}

#[test]
fn ui_section_defaults_the_captions_to_the_package_name() {
    let m = parse(&format!("{HELLO}\n[ui]\nkind = \"avkon\"\n")).unwrap();
    let ui = m.ui.unwrap();
    assert_eq!(ui.kind, UiKind::Avkon);
    assert_eq!(ui.caption, "hello");
    assert_eq!(ui.short_caption, "hello");
    assert_eq!(ui.softkeys, Softkeys::Exit);
    assert_eq!(ui.softkeys.resource(), "R_AVKON_SOFTKEYS_EXIT");
}

#[test]
fn ui_short_caption_falls_back_to_the_caption() {
    let src = format!("{HELLO}\n[ui]\nkind = \"avkon\"\ncaption = \"Bar chart\"\n");
    let ui = parse(&src).unwrap().ui.unwrap();
    assert_eq!(ui.caption, "Bar chart");
    assert_eq!(ui.short_caption, "Bar chart");
    let src = format!("{src}short_caption = \"Bars\"\n");
    assert_eq!(parse(&src).unwrap().ui.unwrap().short_caption, "Bars");
}

/// `softkeys = "options-exit"` would need a menu bar this step does not generate, and
/// a `[ui]` with no `kind` would leave the framework unnamed. Both are refused rather
/// than guessed at.
#[test]
fn ui_rejects_an_empty_caption_an_unknown_kind_and_unobserved_softkeys() {
    reject(&format!(
        "{HELLO}\n[ui]\nkind = \"avkon\"\ncaption = \" \"\n"
    ));
    reject(&format!("{HELLO}\n[ui]\nkind = \"qt\"\n"));
    reject(&format!(
        "{HELLO}\n[ui]\nkind = \"avkon\"\nsoftkeys = \"options-exit\"\n"
    ));
    reject(&format!(
        "{HELLO}\n[ui]\nkind = \"avkon\"\nmenu = \"yes\"\n"
    ));
    reject(&format!("{HELLO}\n[ui]\ncaption = \"Notes\"\n"));
}
