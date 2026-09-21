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
    assert_eq!(ui.left_softkey, "Options");
    assert_eq!(ui.right_softkey, "Exit");
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

/// The Options menu is declared in Rust and the manifest cannot see it, so it no
/// longer guesses: an application that wants the left softkey to open one says so.
#[test]
fn the_left_softkey_is_empty_unless_the_manifest_asks_for_options() {
    let plain = format!("{HELLO}\n[ui]\nkind = \"avkon\"\n");
    assert_eq!(parse(&plain).unwrap().ui.unwrap().softkeys, Softkeys::Exit);
    let with_menu = format!("{plain}softkeys = \"options-exit\"\n");
    assert_eq!(
        parse(&with_menu).unwrap().ui.unwrap().softkeys,
        Softkeys::OptionsExit
    );
}

/// The refusal experiment 91 needed — `options-exit` with no menu bar is an access
/// violation — is now impossible to express rather than refused: `UiResources` emits
/// the menu bar for exactly this value. What is left to check is that the value is
/// accepted on its own, with nothing else in the section.
#[test]
fn the_options_softkey_needs_nothing_else_in_the_manifest() {
    let ui = parse(&format!(
        "{HELLO}\n[ui]\nkind = \"avkon\"\nsoftkeys = \"options-exit\"\n"
    ))
    .unwrap()
    .ui
    .unwrap();
    assert_eq!(ui.softkeys, Softkeys::OptionsExit);
}

#[test]
fn the_softkey_labels_can_be_replaced() {
    let ui = parse(&format!(
        "{HELLO}\n[ui]\nkind = \"avkon\"\n\
         left_softkey = \"Меню\"\nright_softkey = \"Вихід\"\n"
    ))
    .unwrap()
    .ui
    .unwrap();
    assert_eq!(ui.left_softkey, "Меню");
    assert_eq!(ui.right_softkey, "Вихід");
}

/// The menu left the manifest, so a manifest that still carries one is a mistake the
/// person has to be told about rather than a table that is silently ignored.
#[test]
fn ui_rejects_a_leftover_menu_table() {
    reject(&format!(
        "{HELLO}\n[ui]\nkind = \"avkon\"\n[[ui.menu]]\nid = \"more\"\nlabel = \"More\"\n"
    ));
}

#[test]
fn ui_rejects_an_empty_caption_an_unknown_kind_and_a_missing_kind() {
    reject(&format!(
        "{HELLO}\n[ui]\nkind = \"avkon\"\ncaption = \" \"\n"
    ));
    reject(&format!("{HELLO}\n[ui]\nkind = \"qt\"\n"));
    reject(&format!(
        "{HELLO}\n[ui]\nkind = \"avkon\"\nsoftkeys = \"options\"\n"
    ));
    reject(&format!("{HELLO}\n[ui]\ncaption = \"Notes\"\n"));
}
