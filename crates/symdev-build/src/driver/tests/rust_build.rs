use std::path::Path;

use symdev_core::Project;

use symdev_manifest::{Softkeys, UiApp, UiKind};

use super::*;
use crate::rust_sdk::RustSdk;
use crate::ui_resources::UiResources;

fn rust() -> RustBuild {
    RustBuild {
        gcce: fake(),
        sdk: RustSdk::from_env().unwrap(),
        cargo: PathBuf::from("/rustup/bin/cargo"),
        name: "hello".into(),
        ui: None,
    }
}

/// The same project with a `[ui]` section, so every assertion below can be made
/// against the pair and not against one of them.
fn gui() -> RustBuild {
    RustBuild {
        ui: Some(UiResources {
            app: "hello".into(),
            uid3: 0xe735_1c7a,
            ui: UiApp {
                kind: UiKind::Avkon,
                caption: "Hello".into(),
                short_caption: "Hello".into(),
                softkeys: Softkeys::Exit,
            },
            icon: None,
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
    let got = b.link_args(a, None, None, elf, map);
    let mut want = b.gcce.link_args("hello", a, elf, map, &[]);
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
    let cpp = b.gcce.link_args("hello", a, elf, map, &[]);
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
    let project = Project {
        root: PathBuf::from("/p"),
    };
    let shim = b.shim_archive(&project);
    assert_eq!(shim, PathBuf::from("/p/build/shims/libsymrs.a"));
    let got = b.link_args(a, Some(&shim), None, elf, map);
    let archive = got.iter().position(|x| x == &a.display().to_string());
    let at = got.iter().position(|x| x == "/p/build/shims/libsymrs.a");
    let euser = got.iter().position(|x| x == "-l:euser.dso");
    assert_eq!(at, archive.map(|i| i + 1));
    assert!(euser < archive);
}

/// The compiler-runtime archive is searched **after** the application archive and the
/// C++ shim, because both may refer to a routine it defines and an archive is searched
/// only for what is undefined where it appears.
///
/// It is a separate archive on purpose: its entry points are `#[unsafe(no_mangle)]`, so
/// they are global symbols and therefore `--gc-sections` roots in a `-shared` link.
/// Compiled into the application instead, they survived into every program and cost
/// `hello` 756 bytes for code it never calls.
#[test]
fn the_libcall_archive_is_searched_last() {
    let b = rust();
    let (a, elf, map) = (
        Path::new("/p/build/cargo/arm-symbian-e32/release/libhello.a"),
        Path::new("/p/build/hello.elf"),
        Path::new("/p/build/hello.exe.map"),
    );
    let project = Project {
        root: PathBuf::from("/p"),
    };
    let libcalls = b.libcalls().path(&project);
    assert_eq!(
        libcalls,
        PathBuf::from("/p/build/cargo/arm-symbian-e32/libcalls/libsymbian_libcalls.rlib")
    );
    let shim = b.shim_archive(&project);
    let got = b.link_args(a, Some(&shim), Some(&libcalls), elf, map);
    let archive = got.iter().position(|x| x == &a.display().to_string());
    let at_shim = got.iter().position(|x| x == &shim.display().to_string());
    let at_libcalls = got
        .iter()
        .position(|x| x == &libcalls.display().to_string());
    assert_eq!(at_shim, archive.map(|i| i + 1));
    assert_eq!(at_libcalls, archive.map(|i| i + 2));

    // With no C++ shim it still follows the application archive directly.
    let got = b.link_args(a, None, Some(&libcalls), elf, map);
    let archive = got.iter().position(|x| x == &a.display().to_string());
    let at_libcalls = got
        .iter()
        .position(|x| x == &libcalls.display().to_string());
    assert_eq!(at_libcalls, archive.map(|i| i + 1));
}

/// The libcall crate is built by its own cargo invocation, under a profile whose only
/// job is to turn LTO off: under the workspace's `lto = true` the rlib holds LLVM
/// bitcode, which `ld` cannot read.
#[test]
fn the_libcall_crate_is_built_without_lto() {
    let build = rust();
    let args = build.libcalls().cargo_args();
    assert!(args.contains(&"--profile".to_string()));
    assert!(args.contains(&RustSdk::LIBCALLS_PROFILE.to_string()));
    assert!(args.contains(&"-p".to_string()));
    assert!(args.contains(&RustSdk::LIBCALLS_CRATE.to_string()));
    assert!(
        args.iter()
            .any(|a| a.ends_with("symbian-libcalls/Cargo.toml"))
    );
    assert!(args.contains(&"-Zbuild-std=core,alloc".to_string()));
}

#[test]
fn the_archiver_is_derived_from_the_linker() {
    let b = rust();
    let project = Project {
        root: PathBuf::from("/p"),
    };
    let shim = b.shim_archive(&project);
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
    let project = Project {
        root: PathBuf::from("/p"),
    };
    let obj = b.shim_object(&project, &sources[0]);
    assert!(obj.starts_with("/p/build/shims"));
    assert_eq!(obj.extension().unwrap(), "o");

    // The same argv a C++ project's source gets, with the shim directory as the source
    // directory and nothing of the user's project on the include path.
    let got = b.shim_compile_args(&sources[0], &obj, None).unwrap();
    let want = b
        .gcce
        .compile_args(
            &b.sdk.shim_dir(),
            &crate::driver::CompileIncludes::default(),
            &sources[0],
            &obj,
        )
        .unwrap();
    assert_eq!(got, want);
    assert!(got.contains(&"-include".to_string()));
    assert!(got.iter().any(|x| x.ends_with("gcce/gcce.h")));
}

/// The three things `[ui]` adds to the link line, and the one thing it must not move.
#[test]
fn a_gui_link_names_the_vtable_and_the_avkon_libraries() {
    let (a, elf, map) = (
        Path::new("/p/build/cargo/arm-symbian-e32/release/libhello.a"),
        Path::new("/p/build/hello.elf"),
        Path::new("/p/build/hello.exe.map"),
    );
    let console = rust().link_args(a, None, None, elf, map);
    let got = gui().link_args(a, None, None, elf, map);

    // `-u symrs_app_vtbl`, because the reference to it runs from the shim archive
    // back into the Rust archive, which ld has already passed.
    let u = |args: &[String], sym: &str| args.windows(2).any(|w| w[0] == "-u" && w[1] == sym);
    assert!(u(&got, crate::APP_VTBL), "{got:?}");
    assert!(u(&got, crate::E32MAIN), "{got:?}");
    assert!(!u(&console, crate::APP_VTBL), "a console app has no vtable");

    for lib in RustSdk::UI_LIBRARIES {
        let flag = format!("-l:{lib}");
        assert!(got.contains(&flag), "{flag} missing from {got:?}");
        assert!(!console.contains(&flag), "{flag} on a console link");
    }
    // ws32 is not needed: every drawing entry point is a pure virtual.
    assert!(!got.iter().any(|x| x.contains("ws32")), "{got:?}");

    // The 107 KB fix of experiment 77: euser and drtaeabi stay in front of the Rust
    // archive, whatever else the UI adds behind them.
    let pos =
        |args: &[String], needle: &str| args.iter().position(|x| x == needle).unwrap_or(usize::MAX);
    let archive = arg(a);
    assert!(pos(&got, "-l:euser.dso") < pos(&got, &archive), "{got:?}");
    assert!(
        pos(&got, "-l:drtaeabi.dso") < pos(&got, &archive),
        "{got:?}"
    );
    // And the Avkon libraries come after it, where an MMP's LIBRARY list would be.
    assert!(pos(&got, "-l:avkon.dso") > pos(&got, &archive), "{got:?}");
    assert!(pos(&got, "-l:avkon.dso") < pos(&got, "-lsupc++"), "{got:?}");
}

/// The Avkon subclasses are compiled for a `[ui]` project and for no other.
#[test]
fn the_s60_shim_is_only_built_for_a_ui_project() {
    let b = gui();
    let console = b.sdk.shim_sources(false).unwrap();
    let with_ui = b.sdk.shim_sources(true).unwrap();
    assert!(
        !console.iter().any(|s| s.ends_with("symrs_avkon.cpp")),
        "{console:?}"
    );
    assert!(
        with_ui.iter().any(|s| s.ends_with("symrs_avkon.cpp")),
        "{with_ui:?}"
    );
    assert_eq!(with_ui.len(), console.len() + 1);
}

/// What the Avkon headers need beyond the recorded C++ argv: the case-fold overlay
/// (`fbs.h` includes `FbsMessage.h`, which is `fbsmessage.h` on disk) and the UID3
/// the shim's `AppDllUid` returns.
#[test]
fn the_s60_shim_gets_the_casefold_overlay_and_the_uid() {
    let b = gui();
    let sources = b.sdk.shim_sources(true).unwrap();
    let source = sources
        .iter()
        .find(|s| s.ends_with("symrs_avkon.cpp"))
        .unwrap();
    let obj = Path::new("/p/build/shims/symrs_avkon.o");
    let overlay = Path::new("/p/build/sdk-include-casefold");
    let got = b.shim_compile_args(source, obj, Some(overlay)).unwrap();
    let uid = format!("-DSYMRS_UID3=0x{:08x}", b.gcce.uid3);
    assert!(got.contains(&uid), "{uid} missing from {got:?}");
    assert!(got.contains(&arg(overlay)), "{got:?}");
    // The source directory is its own, so `#include "symrs_avkon.h"` resolves.
    assert!(got.contains(&arg(&b.sdk.ui_shim_dir())), "{got:?}");
    // A console project's shim gets neither.
    let plain = rust().shim_compile_args(source, obj, None).unwrap();
    assert!(
        !plain.iter().any(|x| x.starts_with("-DSYMRS_UID3")),
        "{plain:?}"
    );
    assert!(!plain.contains(&arg(overlay)), "{plain:?}");
}
