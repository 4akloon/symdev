use std::path::Path;

use symdev_core::Project;

use symdev_manifest::{Softkeys, UiApp, UiKind};

use super::*;
use crate::rust_sdk::RustSdk;
use crate::ui_resources::UiResources;

pub(super) fn rust() -> RustBuild {
    RustBuild {
        gcce: fake(),
        sdk: RustSdk::at(std::path::Path::new(RustSdk::CHECKOUT.unwrap())).unwrap(),
        cargo: PathBuf::from("/rustup/bin/cargo"),
        rustc: PathBuf::from("/rustup/bin/rustc"),
        name: "hello".into(),
        linker: crate::RustLinker::Gnu,
        ui: None,
        std: false,
    }
}

/// The same project with a `[ui]` section, so every assertion below can be made
/// against the pair and not against one of them.
pub(super) fn gui() -> RustBuild {
    RustBuild {
        ui: Some(UiResources {
            app: "hello".into(),
            uid3: 0xe735_1c7a,
            ui: UiApp {
                kind: UiKind::Avkon,
                caption: "Hello".into(),
                short_caption: "Hello".into(),
                softkeys: Softkeys::Exit,
                left_softkey: "Options".into(),
                right_softkey: "Exit".into(),
            },
            icon: None,
            captions: Vec::new(),
        }),
        ..rust()
    }
}

#[test]
fn cargo_args_are_the_recorded_build_std_invocation() {
    let b = rust();
    let spec = b.sdk.target_spec();
    assert_eq!(
        b.cargo_args(),
        s(&[
            "/rustup/bin/cargo",
            "build",
            "--release",
            "--target",
            &spec.display().to_string(),
            "-Zbuild-std=core,alloc",
            "-Zbuild-std-features=optimize_for_size",
            "-Zjson-target-spec",
            "--target-dir",
            "build/cargo",
        ])
    );
}

#[test]
fn archive_is_under_build_cargo() {
    let b = rust();
    let project = Project {
        root: PathBuf::from("/p"),
    };
    assert_eq!(
        b.archive(&project),
        PathBuf::from("/p/build/cargo/arm-symbian-e32/release/libhello.a")
    );
}

#[test]
fn link_args_add_e32main_gc_sections_and_the_helper_dsos_before_the_archive() {
    let b = rust();
    let (a, elf, map) = (
        Path::new("/p/build/cargo/arm-symbian-e32/release/libhello.a"),
        Path::new("/p/build/hello.elf"),
        Path::new("/p/build/hello.exe.map"),
    );
    let got = b.link_args(&[a.into()], None, None, elf, map).unwrap();
    let mut want = b.gcce.link_args("hello", a, elf, map, &[]).unwrap();
    let at = want.iter().position(|x| x == "_E32Startup").unwrap();
    assert_eq!(want[at - 1], "--entry");
    assert_eq!(&want[at + 1..at + 3], &s(&["-u", "_E32Startup"])[..]);
    want.insert(at + 3, "--gc-sections".into());
    want.insert(at + 4, "-u".into());
    want.insert(at + 5, E32MAIN.into());
    let lib = format!(
        "-L{}",
        b.gcce
            .tools
            .epocroot
            .join("epoc32/release/armv5/lib")
            .display()
    );
    let archive = want
        .iter()
        .position(|x| x == "/p/build/cargo/arm-symbian-e32/release/libhello.a")
        .unwrap();
    want.splice(
        archive..archive,
        [lib.clone(), "-l:euser.dso".into(), "-l:drtaeabi.dso".into()],
    );
    let at = want.iter().position(|x| x == "-lsupc++").unwrap();
    let mut sdk = vec!["--as-needed".to_string()];
    sdk.extend(RustSdk::LIBRARIES.iter().map(|l| format!("-l:{l}")));
    sdk.push("--no-as-needed".into());
    want.splice(at..at, sdk);
    assert_eq!(got, want);
    assert_eq!(got.iter().filter(|x| *x == "-u").count(), 2);

    // Experiment 77: the point of the two DSOs is that they come *before* the archive,
    // so `__aeabi_memclr4` and friends resolve from ROM and compiler_builtins is never
    // pulled. An ordering regression here costs ~7 kB in every Rust binary.
    let euser = got.iter().position(|x| x == "-l:euser.dso").unwrap();
    let drt = got.iter().position(|x| x == "-l:drtaeabi.dso").unwrap();
    let archive = got
        .iter()
        .position(|x| x == "/p/build/cargo/arm-symbian-e32/release/libhello.a")
        .unwrap();
    assert!(euser < archive && drt < archive);
    assert_eq!(got[euser - 1], lib);

    // The C++ line gets neither: it is byte-verified against the SDK's own.
    let cpp = b.gcce.link_args("hello", a, elf, map, &[]).unwrap();
    assert!(!cpp.contains(&"--gc-sections".to_string()));
    let cpp_archive = cpp
        .iter()
        .position(|x| x == "/p/build/cargo/arm-symbian-e32/release/libhello.a")
        .unwrap();
    assert!(!cpp[..cpp_archive].contains(&"-l:drtaeabi.dso".to_string()));
    assert!(!cpp.contains(&"-l:bafl.dso".to_string()));
}

#[test]
fn shim_objects_follow_the_archive_and_keep_the_dso_ordering() {
    let b = rust();
    let (a, elf, map) = (
        Path::new("/p/build/cargo/arm-symbian-e32/release/libhello.a"),
        Path::new("/p/build/hello.elf"),
        Path::new("/p/build/hello.exe.map"),
    );
    let shim = b.shim_archive(Path::new("/p/w"));
    assert_eq!(shim, PathBuf::from("/p/w/shims/libsymrs.a"));
    let got = b
        .link_args(&[a.into()], Some(&shim), None, elf, map)
        .unwrap();
    let archive = got.iter().position(|x| x == &a.display().to_string());
    let at = got.iter().position(|x| x == "/p/w/shims/libsymrs.a");
    let euser = got.iter().position(|x| x == "-l:euser.dso");
    assert_eq!(at, archive.map(|i| i + 1));
    assert!(euser < archive);
}

#[test]
fn the_archiver_is_derived_from_the_linker() {
    let b = rust();
    let shim = b.shim_archive(Path::new("/p/build"));
    let objects = [PathBuf::from("/p/build/shims/symrs_f32.o")];
    assert_eq!(
        b.ar_args(&shim, &objects).unwrap(),
        s(&[
            "/gcc/binutils/bin/arm-none-symbianelf-ar",
            "cr",
            "/p/build/shims/libsymrs.a",
            "/p/build/shims/symrs_f32.o",
        ])
    );
}

#[test]
fn the_sdk_owns_the_shim_sources_and_compiles_them_with_the_cpp_argv() {
    let b = rust();
    let sources = b.sdk.shim_sources(false).unwrap();
    assert!(
        sources.iter().any(|s| s.ends_with("symrs_f32.cpp")),
        "{sources:?}"
    );
    let obj = b.shim_object(Path::new("/p/w"), &sources[0]);
    assert!(obj.starts_with("/p/w/shims"));
    assert_eq!(obj.extension().unwrap(), "o");

    // The same argv a C++ project's source gets, with the shim directory as the source
    // directory and nothing of the user's project on the include path — plus symdev's own
    // section flags in the `OPTION GCCE` slot, right after `-mapcs`.
    let got = b.shim_compile_args(&sources[0], &obj, None).unwrap();
    let mut want = b
        .gcce
        .compile_args(
            &b.sdk.shim_dir(),
            &crate::driver::CompileIncludes::default(),
            &sources[0],
            &obj,
        )
        .unwrap();
    let at = want.iter().position(|a| a == "-mapcs").unwrap() + 1;
    want.splice(at..at, s(&RustBuild::SHIM_OPTIONS));
    assert_eq!(got, want);
    assert!(got.contains(&"-include".to_string()));
    assert!(got.iter().any(|x| x.ends_with("gcce/gcce.h")));
}

#[test]
fn rustc_inputs_stand_where_the_archive_stood_in_order() {
    let b = rust();
    let (elf, map) = (Path::new("/p/w/hello.elf"), Path::new("/p/w/hello.exe.map"));
    let obj = PathBuf::from("/p/out/hello.hello.9136cb57f297e5ab-cgu.0.rcgu.o");
    let cb = PathBuf::from("/p/out/libcompiler_builtins-af926986b8385648.rlib");
    let shim = PathBuf::from("/p/w/shims/libsymrs.a");
    let lc = PathBuf::from("/p/build/cargo/arm-symbian-e32/libcalls/libsymbian_libcalls.rlib");
    let got = b
        .link_args(&[obj.clone(), cb.clone()], Some(&shim), Some(&lc), elf, map)
        .unwrap();
    let mut want = b
        .link_args(std::slice::from_ref(&obj), Some(&shim), Some(&lc), elf, map)
        .unwrap();
    let at = want
        .iter()
        .position(|x| *x == obj.display().to_string())
        .unwrap();
    want.insert(at + 1, cb.display().to_string());
    assert_eq!(got, want);
    let pos = |p: &PathBuf| {
        got.iter()
            .position(|x| *x == p.display().to_string())
            .unwrap()
    };
    let drt = got.iter().position(|x| x == "-l:drtaeabi.dso").unwrap();
    assert!(
        drt < pos(&obj) && pos(&obj) < pos(&cb) && pos(&cb) < pos(&shim) && pos(&shim) < pos(&lc)
    );
}

#[test]
fn a_link_with_no_rust_input_is_an_error() {
    let (elf, map) = (Path::new("/p/w/hello.elf"), Path::new("/p/w/hello.exe.map"));
    let e = rust()
        .link_args(&[], None, None, elf, map)
        .unwrap_err()
        .to_string();
    assert!(e.contains("no Rust object"), "{e}");
}
