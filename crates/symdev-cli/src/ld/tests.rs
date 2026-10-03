use std::ffi::OsString;
use std::path::{Path, PathBuf};

use super::{CargoLinkEnv, CargoOutput, LinkKind, LinkerArgs};

fn argv(name: &str) -> Vec<OsString> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src/ld/testdata")
        .join(name);
    std::fs::read_to_string(path)
        .unwrap()
        .lines()
        .map(OsString::from)
        .collect()
}

fn env(name: &str) -> CargoLinkEnv {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src/ld/testdata")
        .join(name);
    let text = std::fs::read_to_string(path).unwrap();
    CargoLinkEnv::from_pairs(text.lines().filter_map(|l| {
        l.split_once('=')
            .map(|(k, v)| (k.to_string(), v.to_string()))
    }))
}

const OUT: &str = "/work/app/build/cargo/arm-symbian-e32/release/build/app/51c0ecfd4d2a2dcf/out";

#[test]
fn release_binary_gives_the_lto_object_then_compiler_builtins() {
    let a = LinkerArgs::parse(argv("release-bin.argv")).unwrap();
    assert_eq!(
        a.inputs,
        vec![
            PathBuf::from(format!("{OUT}/app.app.6fa0adbb789d939e-cgu.0.rcgu.o")),
            PathBuf::from(
                "/work/app/build/cargo/arm-symbian-e32/release/build/compiler_builtins/af926986b8385648/out/libcompiler_builtins-af926986b8385648.rlib"
            ),
        ]
    );
    assert_eq!(a.output, PathBuf::from(format!("{OUT}/app")));
    assert_eq!(
        a.raw_dylibs,
        vec![PathBuf::from(format!("{OUT}/rustcXXXXXX/raw-dylibs"))]
    );
}

#[test]
fn every_recorded_call_parses_and_keeps_input_order() {
    for (file, inputs) in [
        ("dev-bin.argv", 36),
        ("dev-test.argv", 19),
        ("release-test.argv", 2),
        ("release-example.argv", 2),
    ] {
        let a = LinkerArgs::parse(argv(file)).unwrap();
        assert_eq!(a.inputs.len(), inputs, "{file}");
        assert!(a.inputs[0].to_string_lossy().ends_with(".o"), "{file}");
    }
    let dev = LinkerArgs::parse(argv("dev-bin.argv")).unwrap();
    assert!(dev.inputs[0].ends_with("symbols.o"));
    assert!(
        dev.inputs
            .last()
            .unwrap()
            .to_string_lossy()
            .contains("libcompiler_builtins-")
    );
}

#[test]
fn a_linker_name_without_ld_gets_flavor_gnu_first_and_the_same_line_otherwise() {
    let plain = LinkerArgs::parse(argv("release-bin.argv")).unwrap();
    let flavor = LinkerArgs::parse(argv("release-bin-flavor.argv")).unwrap();
    assert_eq!(plain, flavor);
}

#[test]
fn an_unseen_argument_is_refused_by_name() {
    let mut a = argv("release-bin.argv");
    a.insert(3, OsString::from("--eh-frame-hdr"));
    let e = LinkerArgs::parse(a).unwrap_err().to_string();
    assert!(
        e.contains("`--eh-frame-hdr`") && e.contains("experiment 114"),
        "{e}"
    );
    let mut z = argv("release-bin.argv");
    let at = z.iter().position(|x| x == "noexecstack").unwrap();
    z[at] = OsString::from("relro");
    assert!(
        LinkerArgs::parse(z)
            .unwrap_err()
            .to_string()
            .contains("`-z relro`")
    );
}

#[test]
fn no_output_or_no_input_is_an_error() {
    let a: Vec<OsString> = argv("release-bin.argv")
        .into_iter()
        .filter(|x| !x.to_string_lossy().ends_with("/out/app") && x != "-o")
        .collect();
    assert!(
        LinkerArgs::parse(a)
            .unwrap_err()
            .to_string()
            .contains("no `-o`")
    );
    let b = ["-o", "/x/out/app"].map(OsString::from);
    assert!(
        LinkerArgs::parse(b)
            .unwrap_err()
            .to_string()
            .contains("no object")
    );
}

#[test]
fn the_binary_and_the_test_are_told_apart_by_cargos_variables() {
    assert_eq!(
        LinkKind::of(&env("release-bin.env"), "app").unwrap(),
        LinkKind::Main
    );
    assert_eq!(
        LinkKind::of(&env("dev-bin.env"), "app").unwrap(),
        LinkKind::Main
    );
    assert_eq!(
        LinkKind::of(&env("release-test.env"), "app").unwrap(),
        LinkKind::Test {
            name: "smoke".into()
        }
    );
    assert_eq!(
        LinkKind::of(&env("dev-test.env"), "app").unwrap(),
        LinkKind::Test {
            name: "smoke".into()
        }
    );
}

#[test]
fn a_binary_not_named_after_the_package_is_refused() {
    let e = LinkKind::of(&env("release-example.env"), "app")
        .unwrap_err()
        .to_string();
    assert!(e.contains("`demo`") && e.contains("`app`"), "{e}");
}

#[test]
fn neither_a_binary_nor_a_test_is_refused() {
    let e = LinkKind::of(&CargoLinkEnv::from_pairs([]), "app")
        .unwrap_err()
        .to_string();
    assert!(
        e.contains("CARGO_BIN_NAME") && e.contains("CARGO_TARGET_TMPDIR"),
        "{e}"
    );
}

#[test]
fn the_output_names_the_profile_directory_cargo_links_the_binary_into() {
    let o = CargoOutput::of(Path::new(&format!("{OUT}/app"))).unwrap();
    assert_eq!(
        o.profile_dir(),
        Path::new("/work/app/build/cargo/arm-symbian-e32/release")
    );
    assert_eq!(o.sisx(), PathBuf::from(format!("{OUT}/app.sisx")));
    assert_eq!(o.work_dir(), PathBuf::from(format!("{OUT}/app.symdev")));
    assert_eq!(o.record(), PathBuf::from(format!("{OUT}/app.symdev.toml")));
}

#[test]
fn an_output_outside_cargos_observed_layout_is_refused() {
    for odd in [
        "/work/app/build/cargo/arm-symbian-e32/release/deps/app-0123456789abcdef",
        "/tmp/app",
        "app",
    ] {
        let e = CargoOutput::of(Path::new(odd)).unwrap_err().to_string();
        assert!(e.contains("not observed"), "{odd}: {e}");
    }
}
