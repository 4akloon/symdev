//! What `[ui]` accepts, what it defaults and what it refuses.
use crate::tests::{HELLO, reject};
use crate::{CommandId, Softkeys, UiKind, parse};

/// A `[ui]` with one menu item, which is what turns the Options softkey on.
fn with_menu(extra: &str) -> String {
    format!(
        "{HELLO}\n[ui]\nkind = \"avkon\"\n{extra}\n\
         [[ui.menu]]\nid = \"more\"\nlabel = \"More bars\"\n"
    )
}

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
    assert!(ui.menu.is_empty());
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

/// No menu means nothing to open, so the left softkey stays empty; a menu means the
/// opposite. Neither has to be written down.
#[test]
fn the_softkeys_follow_the_menu() {
    let without = format!("{HELLO}\n[ui]\nkind = \"avkon\"\n");
    assert_eq!(
        parse(&without).unwrap().ui.unwrap().softkeys,
        Softkeys::Exit
    );
    assert_eq!(
        parse(&with_menu("")).unwrap().ui.unwrap().softkeys,
        Softkeys::OptionsExit
    );
}

#[test]
fn a_menu_item_carries_the_command_its_name_hashes_to() {
    let ui = parse(&with_menu("")).unwrap().ui.unwrap();
    assert_eq!(ui.menu.len(), 1);
    assert_eq!(ui.menu[0].name, "more");
    assert_eq!(ui.menu[0].label, "More bars");
    assert_eq!(ui.menu[0].command, CommandId::of("more"));
}

#[test]
fn the_softkey_labels_can_be_replaced() {
    let ui = parse(&with_menu(
        "left_softkey = \"Меню\"\nright_softkey = \"Вихід\"",
    ))
    .unwrap()
    .ui
    .unwrap();
    assert_eq!(ui.left_softkey, "Меню");
    assert_eq!(ui.right_softkey, "Вихід");
}

/// `options-exit` with no menu is the one combination that does not merely look
/// wrong: the framework dereferences the menu bar the instant the left softkey is
/// pressed and the application dies with an access violation (experiment 91).
#[test]
fn ui_rejects_the_options_softkey_without_a_menu() {
    reject(&format!(
        "{HELLO}\n[ui]\nkind = \"avkon\"\nsoftkeys = \"options-exit\"\n"
    ));
}

#[test]
fn ui_rejects_a_menu_item_with_no_name_no_label_or_a_repeated_name() {
    reject(&format!(
        "{HELLO}\n[ui]\nkind = \"avkon\"\n[[ui.menu]]\nlabel = \"More\"\n"
    ));
    reject(&format!(
        "{HELLO}\n[ui]\nkind = \"avkon\"\n[[ui.menu]]\nid = \"more\"\n"
    ));
    reject(&format!(
        "{HELLO}\n[ui]\nkind = \"avkon\"\n\
         [[ui.menu]]\nid = \"more\"\nlabel = \"More\"\n\
         [[ui.menu]]\nid = \"more\"\nlabel = \"Also more\"\n"
    ));
}

#[test]
fn ui_rejects_an_empty_caption_an_unknown_kind_and_a_missing_kind() {
    reject(&format!(
        "{HELLO}\n[ui]\nkind = \"avkon\"\ncaption = \" \"\n"
    ));
    reject(&format!("{HELLO}\n[ui]\nkind = \"qt\"\n"));
    reject(&format!(
        "{HELLO}\n[ui]\nkind = \"avkon\"\nmenu = \"yes\"\n"
    ));
    reject(&format!("{HELLO}\n[ui]\ncaption = \"Notes\"\n"));
}

#[test]
fn a_menu_id_becomes_an_upper_case_constant() {
    let src = with_menu("") + "[[ui.menu]]\nid = \"new-note\"\nlabel = \"New\"\n";
    let ui = parse(&src).unwrap().ui.unwrap();
    assert_eq!(ui.menu[0].constant(), "MORE");
    assert_eq!(ui.menu[1].constant(), "NEW_NOTE");
}

#[test]
fn ui_rejects_an_id_that_cannot_be_a_constant() {
    for id in ["More", "2fast", "new note", "меню", "-x"] {
        let src = format!(
            "{HELLO}\n[ui]\nkind = \"avkon\"\n[[ui.menu]]\nid = \"{id}\"\nlabel = \"x\"\n"
        );
        reject(&src);
        let err = parse(&src).unwrap_err().to_string();
        assert!(err.contains("lower case ASCII word"), "{id}: {err}");
    }
}

#[test]
fn ui_rejects_two_ids_that_would_be_one_constant() {
    let src = with_menu("") + "[[ui.menu]]\nid = \"new-note\"\nlabel = \"a\"\n[[ui.menu]]\nid = \"new_note\"\nlabel = \"b\"\n";
    reject(&src);
    let err = parse(&src).unwrap_err().to_string();
    assert!(err.contains("`menu::NEW_NOTE`"), "{err}");
}
