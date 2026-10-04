use super::*;
use crate::cli::Lang;
use crate::scaffold::create_project;

fn scratch() -> PathBuf {
    let p = std::env::temp_dir().join(format!(
        "symdev-scaffold-rust-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&p).unwrap();
    p
}

#[test]
fn rust_project_has_cargo_files_and_no_mmp() {
    let dir = scratch();
    let checkout = || RustSdk::at(Path::new(RustSdk::CHECKOUT.unwrap()));
    let root = create_project(&dir, "hello", Template::Console, Lang::Rust, checkout).unwrap();
    let sdk = checkout().unwrap();
    let read = |p: &str| std::fs::read_to_string(root.join(p)).unwrap();
    assert!(read("symdev.toml").contains("name = \"rust\""));
    assert!(read("symdev.toml").contains("uid3 = \"0xef9f2cab\""));
    let cargo = read("Cargo.toml");
    assert!(
        !cargo.contains("staticlib") && !cargo.contains("autobins"),
        "{cargo}"
    );
    assert!(
        cargo.contains("[[bin]]\nname = \"hello\"\npath = \"src/main.rs\"\ntest = false\n"),
        "{cargo}"
    );
    assert!(
        cargo.contains("[[test]]\nname = \"smoke\"\nharness = false\n"),
        "{cargo}"
    );
    assert!(
        cargo.contains(
            "symbian-test = { path = \"build/rust-sdk/symbian-rs/crates/symbian-test\" }"
        )
    );
    let config = read(".cargo/config.toml");
    for line in [
        "panic-abort-tests = true",
        "[target.arm-symbian-e32]",
        "linker = \"symdev-ld\"",
        "runner = \"symdev run --exe\"",
    ] {
        assert!(config.contains(line), "{line}: {config}");
    }
    assert!(read("src/main.rs").contains("#![no_main]"));
    let smoke = read("tests/smoke.rs");
    assert!(
        smoke.contains("#[symbian_test::tests]") && smoke.contains("#![no_main]"),
        "{smoke}"
    );
    assert_eq!(
        read("rust-toolchain.toml"),
        sdk_file(&sdk, "rust-toolchain.toml")
    );
    assert_eq!(read("src/main.rs"), RustSdk::HELLO_MAIN);
    assert!(!root.join("group").exists());
    assert!(!root.join("bld.inf").exists());
}

fn sdk_file(sdk: &RustSdk, file: &str) -> String {
    std::fs::read_to_string(sdk.root().join(file)).unwrap()
}

/// The project names the SDK only through `build/rust-sdk`, which `symdev new` links
/// and every `symdev build` keeps pointing at the SDK it resolved (experiment 110).
#[test]
fn rust_project_names_the_sdk_through_build_rust_sdk() {
    let dir = scratch();
    let checkout = || RustSdk::at(Path::new(RustSdk::CHECKOUT.unwrap()));
    let root = create_project(&dir, "hello", Template::Console, Lang::Rust, checkout).unwrap();
    let sdk = checkout().unwrap();
    let read = |p: &str| std::fs::read_to_string(root.join(p)).unwrap();
    let cargo = read("Cargo.toml");
    for name in ["symbian-core", "symbian-std"] {
        let line = format!("{name} = {{ path = \"build/rust-sdk/symbian-rs/crates/{name}\" }}");
        assert!(cargo.contains(&line), "{cargo}");
    }
    let config = read(".cargo/config.toml");
    let target = "target = \"build/rust-sdk/symbian-rs/targets/arm-symbian-e32.json\"";
    assert!(config.contains(target), "{config}");
    // A fresh clone has no link until a build makes it (review 0.2.0, minor 4).
    assert!(config.contains("run `symdev build` once"), "{config}");
    let tree = sdk.root().parent().unwrap().display().to_string();
    assert!(!cargo.contains(&tree) && !config.contains(&tree));
    let link = std::fs::read_link(root.join("build/rust-sdk")).unwrap();
    assert_eq!(link, sdk.root().parent().unwrap());
    assert!(read("build/.gitignore").lines().any(|l| l == "*"));
}

#[test]
fn rust_gui_template_is_a_todo() {
    let dir = scratch();
    let sdk = || panic!("the GUI template is refused before the SDK is looked for");
    let err = create_project(&dir, "notes", Template::Gui, Lang::Rust, sdk).unwrap_err();
    assert!(err.to_string().starts_with("TODO:"), "{err}");
}
