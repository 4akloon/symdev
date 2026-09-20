use std::path::Path;

use super::*;

fn args_for(source: &str) -> Vec<String> {
    fake()
        .compile_args(
            Path::new("/proj"),
            &CompileIncludes::default(),
            Path::new(source),
            Path::new("/proj/build/o.o"),
        )
        .unwrap()
}

#[test]
fn c_sources_go_through_the_c_front_end() {
    let args = args_for("/proj/game.c");
    let x = args.iter().position(|a| a == "-x").unwrap();
    assert_eq!(args[x + 1], "c");
    // Both leniency flags are C++-only: `-fpermissive` is diagnosed by cc1 and
    // narrowing is a C++ conversion rule (experiment 59).
    assert!(!args.iter().any(|a| a == "-fpermissive"));
    assert!(!args.iter().any(|a| a == "-Wno-narrowing"));
    // Everything else the SDK passes is language-independent and stays.
    for kept in ["-O2", "-fexceptions", "-march=armv5t", "-nostdinc", "-c"] {
        assert!(args.iter().any(|a| a == kept), "missing {kept}");
    }
    assert!(args.iter().any(|a| a == "-D__GCCE__"));
    assert_eq!(args.last().map(String::as_str), Some("/proj/game.c"));
}

#[test]
fn cpp_sources_keep_the_recorded_argv() {
    let args = args_for("/proj/gui.cpp");
    // `-x c++` is not passed: the recorded experiment-5 argv does not carry it.
    assert!(!args.iter().any(|a| a == "-x"));
    assert!(args.iter().any(|a| a == "-fpermissive"));
    assert!(args.iter().any(|a| a == "-Wno-narrowing"));
}

#[test]
fn the_dialect_flags_sit_in_the_same_slot() {
    let cpp = args_for("/proj/gui.cpp");
    let c = args_for("/proj/game.c");
    let at = |v: &[String], s: &str| v.iter().position(|a| a == s).unwrap();
    let slot = at(&cpp, "-msoft-float") + 1;
    assert_eq!(slot, at(&c, "-msoft-float") + 1);
    assert_eq!(&cpp[slot..slot + 2], s(&["-fpermissive", "-Wno-narrowing"]));
    assert_eq!(&c[slot..slot + 2], s(&["-x", "c"]));
    assert_eq!(cpp[slot + 2], "-D__SYMBIAN32__");
    assert_eq!(c[slot + 2], "-D__SYMBIAN32__");
}

#[test]
fn assembly_sources_are_refused_by_name() {
    for source in ["/proj/arm.s", "/proj/arm.S", "/proj/fast.cia"] {
        let err = fake()
            .compile_args(
                Path::new("/proj"),
                &CompileIncludes::default(),
                Path::new(source),
                Path::new("/proj/build/o.o"),
            )
            .unwrap_err()
            .to_string();
        assert!(err.contains("TODO"), "{source}: {err}");
        assert!(err.contains(source), "{source}: {err}");
    }
}

#[test]
fn the_language_comes_from_the_extension() {
    assert_eq!(
        SourceLanguage::of(Path::new("a/b/game.c")).unwrap(),
        SourceLanguage::C
    );
    assert_eq!(
        SourceLanguage::of(Path::new("a/b/gui.cpp")).unwrap(),
        SourceLanguage::Cpp
    );
    assert!(SourceLanguage::of(Path::new("a/b/README")).is_err());
}
