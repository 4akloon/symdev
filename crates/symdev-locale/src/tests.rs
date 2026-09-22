use std::path::Path;

use super::{Language, Locales, Table};

fn dir_with(files: &[(&str, &str)]) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    for (name, text) in files {
        std::fs::write(dir.path().join(name), text).unwrap();
    }
    dir
}

fn err(dir: &Path) -> String {
    match Locales::load(dir) {
        Ok(_) => panic!("expected an error"),
        Err(e) => e.to_string(),
    }
}

#[test]
fn a_language_is_found_by_its_file_name_word() {
    assert_eq!(Language::named("ukrainian").unwrap().code, 93);
    assert_eq!(Language::named("ukrainian").unwrap().suffix(), "r93");
    assert_eq!(Language::named("french").unwrap().suffix(), "r02");
    assert_eq!(Language::named("english_apac").unwrap().suffix(), "r129");
    assert!(Language::named("klingon").is_none());
    assert!(Language::named("none").is_none());
    assert!(Language::named("test").is_none());
}

#[test]
fn a_file_is_key_equals_quoted_string_per_line() {
    let t = Table::parse(
        "x.toml",
        "# c\n\ngreeting = \"Привіт\"\nok = \"a\\\"b\\\\c\\nd\\u0041\"\n",
    )
    .unwrap();
    assert_eq!(t.entries["greeting"], "Привіт");
    assert_eq!(t.entries["ok"], "a\"b\\c\nd\u{41}");
}

#[test]
fn anything_outside_the_subset_is_refused_with_its_line() {
    for (text, want) in [
        ("[app]\n", "x.toml:1"),
        ("a = [1]\n", "x.toml:1"),
        ("a = 1\n", "x.toml:1"),
        ("\nBig = \"x\"\n", "x.toml:2"),
        ("a = \"x\\u0000\"\n", "x.toml:1"),
        ("a = \"x\"\na = \"y\"\n", "x.toml:2"),
        ("a = \"unterminated\n", "x.toml:1"),
    ] {
        let e = Table::parse("x.toml", text).unwrap_err().to_string();
        assert!(e.contains(want), "{text:?} gave {e}");
    }
}

#[test]
fn no_directory_is_no_localisation() {
    let dir = tempfile::tempdir().unwrap();
    assert!(
        Locales::load(&dir.path().join("locales"))
            .unwrap()
            .is_none()
    );
}

#[test]
fn the_default_file_is_required() {
    let dir = dir_with(&[("french.toml", "a = \"b\"\n")]);
    assert!(err(dir.path()).contains("default.toml"));
}

#[test]
fn every_variant_has_exactly_the_default_keys() {
    let dir = dir_with(&[
        ("default.toml", "a = \"A\"\nb = \"B\"\n"),
        ("french.toml", "a = \"a\"\n"),
    ]);
    let e = err(dir.path());
    assert!(e.contains("french.toml") && e.contains("`b`"), "{e}");

    let dir = dir_with(&[
        ("default.toml", "a = \"A\"\n"),
        ("french.toml", "a = \"a\"\nz = \"z\"\n"),
    ]);
    let e = err(dir.path());
    assert!(e.contains("french.toml") && e.contains("`z`"), "{e}");
}

#[test]
fn a_file_not_named_after_a_language_is_refused() {
    let dir = dir_with(&[
        ("default.toml", "a = \"A\"\n"),
        ("frnech.toml", "a = \"a\"\n"),
    ]);
    assert!(err(dir.path()).contains("frnech"));
}

#[test]
fn the_caption_translates_the_manifest_and_is_not_a_string() {
    let dir = dir_with(&[("default.toml", "caption = \"X\"\n")]);
    assert!(err(dir.path()).contains("caption"));

    let dir = dir_with(&[
        ("default.toml", "b = \"B\"\na = \"A\"\n"),
        (
            "french.toml",
            "caption = \"Barres\"\na = \"a\"\nb = \"b\"\n",
        ),
    ]);
    let l = Locales::load(dir.path()).unwrap().unwrap();
    assert_eq!(l.keys(), vec!["a", "b"]);
    assert_eq!(l.index("a"), Some(2));
    assert_eq!(l.index("b"), Some(3));
    assert_eq!(l.index("caption"), None);
    assert_eq!(l.variants[0].1.caption(), Some("Barres"));
    assert_eq!(l.variants[0].0.code, 2);
}
