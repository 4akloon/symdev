use std::collections::BTreeMap;

use symdev_locale::{Language, Locales, Table};

use super::StringsResources;

fn table(pairs: &[(&str, &str)]) -> Table {
    Table {
        entries: pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect::<BTreeMap<_, _>>(),
    }
}

fn strings(default: Table) -> StringsResources {
    StringsResources {
        app: "demo".into(),
        locales: Locales {
            default,
            variants: Vec::new(),
        },
    }
}

/// The generated source through the real native preprocessor and compiler.
fn compile(rss: &str) -> symdev_rcomp::RscCompiled {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("s.rss");
    std::fs::write(&path, rss).unwrap();
    let rpp = symdev_rcomp::CPreprocessor::for_rss(&[], &[])
        .run(&path)
        .unwrap();
    symdev_rcomp::Rcomp::compile(&rpp, "s.rss").unwrap()
}

#[test]
fn the_source_is_a_signature_then_one_buf8_per_key_in_key_order() {
    let s = strings(table(&[("ok", "ok"), ("greeting", "Hello")]));
    let rss = s.rss(&s.locales.default);
    let greeting = rss.find("\"Hello\"").unwrap();
    let ok = rss.find("\"ok\"").unwrap();
    assert!(greeting < ok, "keys are compiled in byte order:\n{rss}");
    assert!(rss.contains("NAME STRS"));
    assert!(rss.contains("RESOURCE SYMDEV_SIG { }"));
    assert!(
        !rss.contains("#include"),
        "must build without SDK headers:\n{rss}"
    );
}

#[test]
fn every_value_reaches_the_compiled_resource_as_its_exact_utf8() {
    let values = [
        ("a_plain", "Hello from Rust"),
        ("b_quote", "say \"hi\" \\ back"),
        ("c_latin", "d'accord é ü"),
        ("d_cyrillic", "Привіт з Rust"),
        ("e_controls", "line\nnext\ttab"),
        ("f_high", "\u{9f}\u{80}\u{ff}"),
    ];
    let s = strings(table(&values));
    let compiled = compile(&s.rss(&s.locales.default));
    for (key, value) in values {
        let index = s.locales.index(key).unwrap();
        let resource = &compiled.resources[usize::from(index) - 1];
        assert_eq!(
            resource.data.uncompressed(),
            value.as_bytes(),
            "{key} at index {index}"
        );
    }
    // 8-bit text is never compressed (`rcomp-spec.md` §3.1), so each value's UTF-8 must
    // also be in the file the phone reads, byte for byte.
    let file = compiled.rsc_bytes().unwrap();
    for (key, value) in values {
        let bytes = value.as_bytes();
        assert!(
            file.windows(bytes.len()).any(|w| w == bytes),
            "{key}'s UTF-8 is not in the .rsc"
        );
    }
    // Index 1 is the signature: LONG 4, then the self link.
    assert_eq!(
        &compiled.resources[0].data.uncompressed()[..4],
        &4u32.to_le_bytes()
    );
}

#[test]
fn files_are_named_after_the_app_and_the_language() {
    let s = strings(table(&[("a", "b")]));
    let dir = std::path::Path::new("/p/build");
    let french = Language::named("french").unwrap();
    assert_eq!(s.rsc_path(dir, None), dir.join("demo_strings.rsc"));
    assert_eq!(s.rsc_path(dir, Some(french)), dir.join("demo_strings.r02"));
    assert_eq!(s.dest(None), "!:\\resource\\apps\\demo_strings.rsc");
    assert_eq!(s.dest(Some(french)), "!:\\resource\\apps\\demo_strings.r02");
}
