use std::path::Path;

use symdev_core::Project;

use super::*;
use crate::rust_sdk::RustSdk;

fn rust() -> RustBuild {
    RustBuild {
        gcce: fake(),
        sdk: RustSdk::from_env().unwrap(),
        cargo: PathBuf::from("/rustup/bin/cargo"),
        name: "hello".into(),
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
    let got = b.link_args(a, &[], elf, map);
    let libraries: Vec<String> = RustSdk::LIBRARIES.iter().map(|l| (*l).into()).collect();
    let mut want = b.gcce.link_args("hello", a, elf, map, &libraries);
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
    let shims = [PathBuf::from("/p/build/shims/symrs_f32.o")];
    let got = b.link_args(a, &shims, elf, map);
    let archive = got.iter().position(|x| x == &a.display().to_string());
    let shim = got.iter().position(|x| x == "/p/build/shims/symrs_f32.o");
    let euser = got.iter().position(|x| x == "-l:euser.dso");
    assert_eq!(shim, archive.map(|i| i + 1));
    assert!(euser < archive);
}

#[test]
fn the_sdk_owns_the_shim_sources_and_compiles_them_with_the_cpp_argv() {
    let b = rust();
    let sources = b.sdk.shim_sources().unwrap();
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
    let got = b.shim_compile_args(&sources[0], &obj).unwrap();
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
