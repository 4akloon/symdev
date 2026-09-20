use std::path::{Path, PathBuf};

use super::*;

#[test]
fn the_compat_header_follows_gcce_h() {
    let d = fake();
    let inc = CompileIncludes {
        user: Vec::new(),
        system: Vec::new(),
        prefix: vec![PathBuf::from("/p/build/symdev-gcce-compat.h")],
    };
    let args = d
        .compile_args(
            Path::new("/p"),
            &inc,
            Path::new("/p/gui.cpp"),
            Path::new("/p/build/gui.o"),
        )
        .unwrap();
    let gcce = args
        .iter()
        .position(|a| a == "/sdk/epoc32/include/gcce/gcce.h")
        .unwrap();
    let compat = args
        .iter()
        .position(|a| a == "/p/build/symdev-gcce-compat.h")
        .unwrap();
    assert_eq!(args[gcce - 1], "-include");
    assert_eq!(args[compat - 1], "-include");
    assert!(gcce < compat, "gcce.h must be included first");
    // Nothing between them: the repair lands immediately after what it repairs.
    assert_eq!(gcce + 2, compat);
}

#[test]
fn without_a_prefix_header_the_argv_is_unchanged() {
    let d = fake();
    let args = d
        .compile_args(
            Path::new("/p"),
            &CompileIncludes::default(),
            Path::new("/p/gui.cpp"),
            Path::new("/p/build/gui.o"),
        )
        .unwrap();
    assert_eq!(args.iter().filter(|a| *a == "-include").count(), 1);
}

#[test]
fn the_compat_header_repairs_the_macros_without_retyping_va_list() {
    // Only the six directives count; the rest of the file is the comment that
    // explains why it exists.
    let code: Vec<&str> = GcceCompat::SOURCE
        .lines()
        .map(str::trim)
        .filter(|l| l.starts_with('#'))
        .collect();
    assert_eq!(code.len(), 6, "{code:?}");
    for (i, name) in ["va_start", "va_arg", "va_end"].iter().enumerate() {
        assert_eq!(code[i], format!("#undef {name}"));
        assert!(
            code[i + 3].starts_with(&format!("#define {name}(ap")),
            "{name}"
        );
        // The builtins must see the whole `ap`, not `gcce.h`'s `ap.__ap` member.
        assert!(
            code[i + 3].contains("*(__builtin_va_list *)&(ap)"),
            "{name}"
        );
        assert!(!code[i + 3].contains("ap.__ap"), "{name}");
    }
    // The ABI must not move: nothing here declares a type or touches VA_LIST.
    assert!(
        !code
            .iter()
            .any(|l| l.contains("typedef") || l.contains("VA_"))
    );
}

#[test]
fn ensure_writes_the_header_into_the_build_dir() {
    let dir = std::env::temp_dir().join(format!("symdev-compat-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let compat = GcceCompat::ensure(&dir).unwrap();
    assert_eq!(compat.header(), dir.join("symdev-gcce-compat.h"));
    assert_eq!(
        std::fs::read_to_string(compat.header()).unwrap(),
        GcceCompat::SOURCE
    );
    // Rewritten on every build, not only the first.
    GcceCompat::ensure(&dir).unwrap();
    std::fs::remove_dir_all(&dir).unwrap();
}
