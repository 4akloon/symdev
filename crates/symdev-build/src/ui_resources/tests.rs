//! The generated `.rss` text, against `examples/gui/data/*.rss` — the pair that is
//! known to install and run.
use std::path::{Path, PathBuf};

use symdev_manifest::{CommandId, MenuItem, Softkeys, UiApp, UiKind};

use super::UiResources;

fn gui(icon: bool) -> UiResources {
    UiResources {
        app: "gui".into(),
        uid3: 0xe735_1c20,
        ui: UiApp {
            kind: UiKind::Avkon,
            caption: "gui".into(),
            short_caption: "gui".into(),
            softkeys: Softkeys::Exit,
            left_softkey: "Options".into(),
            right_softkey: "Exit".into(),
            menu: Vec::new(),
        },
        icon: icon.then(|| PathBuf::from("/p/gfx/gui.svg")),
    }
}

/// The same application with a two-item Options menu, which is what turns the left
/// softkey on.
fn with_menu() -> UiResources {
    let mut r = gui(false);
    r.ui.softkeys = Softkeys::OptionsExit;
    r.ui.menu = ["more", "fewer"]
        .into_iter()
        .map(|name| MenuItem {
            name: name.into(),
            label: format!("{name} bars"),
            command: CommandId::of(name),
        })
        .collect();
    r
}

/// Every line `examples/gui/data/gui.rss` has, in the same order, with the caption
/// and the icon substituted. That file is the one whose compiled resource runs.
#[test]
fn the_application_resource_matches_the_working_example() {
    let rss = gui(true).app_rss();
    for expected in [
        "NAME APPR",
        "#include <eikon.rh>",
        "#include <avkon.rh>",
        "#include <appinfo.rh>",
        "RESOURCE RSS_SIGNATURE { }",
        "RESOURCE TBUF { buf = \"\"; }",
        "RESOURCE EIK_APP_INFO",
        "cba = r_symrs_cba;",
        "RESOURCE LOCALISABLE_APP_INFO r_app_localisable_app_info",
        "short_caption = \"gui\";",
        "caption_and_icon = CAPTION_AND_ICON_INFO",
        "caption = \"gui\";",
        "number_of_icons = 1;",
        "icon_file = \"\\\\resource\\\\apps\\\\gui_aif.mif\";",
    ] {
        assert!(rss.contains(expected), "missing {expected:?} in:\n{rss}");
    }
}

/// An application with no `[symbian] icon` must not name a `.mif` the package does
/// not carry: that is a blank square in the menu, not a missing-file error.
#[test]
fn without_an_icon_the_resource_declares_none() {
    let rss = gui(false).app_rss();
    assert!(rss.contains("number_of_icons = 0;"), "{rss}");
    assert!(!rss.contains("icon_file"), "{rss}");
}

/// What `Rsc::registration` cannot write, and the whole reason this stage exists.
#[test]
fn the_registration_resource_points_at_the_localisable_one() {
    let rss = gui(true).reg_rss();
    for expected in [
        "#include <gui.rsg>",
        "UID2 KUidAppRegistrationResourceFile",
        "UID3 0xe7351c20",
        "app_file = \"gui\";",
        "localisable_resource_file = \"\\\\resource\\\\apps\\\\gui\";",
        "localisable_resource_id = R_APP_LOCALISABLE_APP_INFO;",
    ] {
        assert!(rss.contains(expected), "missing {expected:?} in:\n{rss}");
    }
}

#[test]
fn a_caption_with_a_quote_or_a_backslash_stays_one_string_literal() {
    let mut r = gui(false);
    r.ui.caption = "He said \"hi\"\\".into();
    r.ui.short_caption = r.ui.caption.clone();
    let rss = r.app_rss();
    assert!(
        rss.contains("caption = \"He said \\\"hi\\\"\\\\\";"),
        "{rss}"
    );
}

/// The registration goes where the application architecture server reads them from,
/// and the localisable resource beside the other application resources.
#[test]
fn the_install_destinations_are_the_ones_the_framework_reads() {
    let r = gui(true);
    assert_eq!(r.app_rsc_dest(), "!:\\resource\\apps\\gui.rsc");
    assert_eq!(
        r.reg_rsc_dest(),
        "!:\\private\\10003a3f\\import\\apps\\gui_reg.rsc"
    );
    let build = Path::new("/p/build");
    assert_eq!(r.app_rss_path(build), Path::new("/p/build/gui.rss"));
    assert_eq!(r.rsg_path(build), Path::new("/p/build/gui.rsg"));
    let artifacts = r.artifacts(build);
    assert_eq!(artifacts.len(), 2);
    assert_eq!(artifacts[1].path, Path::new("/p/build/gui_reg.rsc"));
}

/// Without a menu there is nothing for the left softkey to open, so it carries
/// Avkon's own "does nothing" id and no label, and no menu bar is named.
#[test]
fn without_a_menu_the_left_softkey_is_empty_and_no_menu_bar_is_named() {
    let rss = gui(false).app_rss();
    assert!(
        rss.contains("CBA_BUTTON { id = EAknSoftkeyEmpty; txt = \"\"; }"),
        "{rss}"
    );
    assert!(
        rss.contains("CBA_BUTTON { id = EEikCmdExit; txt = \"Exit\"; }"),
        "{rss}"
    );
    assert!(!rss.contains("menubar"), "{rss}");
    assert!(!rss.contains("MENU_"), "{rss}");
}

/// The id on the right button is `EEikCmdExit` and never one of Avkon's own CBA
/// resources. That is the whole of the softkey fix: with `cba = R_AVKON_SOFTKEYS_EXIT`
/// the command that reached `HandleCommandL` on this ROM was 3001
/// (`EAknSoftkeyBack`), which nothing handled (experiment 91).
#[test]
fn the_menu_and_its_softkey_are_generated_with_our_own_command_ids() {
    let rss = with_menu().app_rss();
    for expected in [
        "menubar = r_symrs_menubar;",
        "cba = r_symrs_cba;",
        "CBA_BUTTON { id = EAknSoftkeyOptions; txt = \"Options\"; }",
        "CBA_BUTTON { id = EEikCmdExit; txt = \"Exit\"; }",
        "RESOURCE MENU_BAR r_symrs_menubar",
        "MENU_TITLE { menu_pane = r_symrs_menupane; txt = \"Options\"; }",
        "RESOURCE MENU_PANE r_symrs_menupane",
        "MENU_ITEM { command = 0x41e0; txt = \"more bars\"; },",
        "MENU_ITEM { command = 0x7612; txt = \"fewer bars\"; }",
    ] {
        assert!(rss.contains(expected), "missing {expected:?} in:\n{rss}");
    }
    assert!(!rss.contains("R_AVKON_SOFTKEYS"), "{rss}");
}

/// `EIK_APP_INFO` has to stay the third resource, so it names the button group and
/// the menu bar before either is declared. `rcomp` resolves the forward reference —
/// which is also how every hand-written S60 `.rss` is laid out.
#[test]
fn the_app_info_stays_third_and_refers_forward() {
    let rss = with_menu().app_rss();
    let at = |needle: &str| rss.find(needle).unwrap_or_else(|| panic!("no {needle:?}"));
    assert!(at("RESOURCE RSS_SIGNATURE") < at("RESOURCE TBUF"));
    assert!(at("RESOURCE TBUF") < at("RESOURCE EIK_APP_INFO"));
    assert!(at("RESOURCE EIK_APP_INFO") < at("RESOURCE CBA r_symrs_cba"));
    assert!(at("RESOURCE CBA r_symrs_cba") < at("RESOURCE MENU_BAR r_symrs_menubar"));
    assert!(at("RESOURCE MENU_BAR r_symrs_menubar") < at("RESOURCE MENU_PANE r_symrs_menupane"));
    assert!(at("RESOURCE MENU_PANE r_symrs_menupane") < at("RESOURCE LOCALISABLE_APP_INFO"));
}

/// A label is text a person wrote, so it goes through the same escaping the caption
/// does rather than ending the string literal early.
#[test]
fn a_menu_label_and_a_softkey_label_are_escaped() {
    let mut r = with_menu();
    r.ui.menu[0].label = "say \"hi\"".into();
    r.ui.left_softkey = "a\\b".into();
    let rss = r.app_rss();
    assert!(rss.contains("txt = \"say \\\"hi\\\"\";"), "{rss}");
    assert!(rss.contains("txt = \"a\\\\b\";"), "{rss}");
}
