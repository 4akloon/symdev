//! `symdev new <name> --language rust`: a Cargo project on the Rust SDK (experiment 65).
//! No `bld.inf`, no `.mmp`: `RustBuild` runs cargo and then the recorded link line.
use std::path::{Path, PathBuf};

use symdev_build::RustSdk;
use symdev_core::Error;

use crate::cli::Template;
use crate::scaffold::{io_err, uid3_hex};

pub fn write_rust(root: &Path, name: &str, template: Template) -> Result<PathBuf, Error> {
    if template != Template::Console {
        return Err(Error::Other(
            "TODO: --language rust supports only --template console (the GUI template \
             needs the Avkon framework, which the Rust SDK does not reach yet)"
                .into(),
        ));
    }
    let sdk = RustSdk::from_env()?;
    std::fs::create_dir_all(root.join("src")).map_err(io_err)?;
    std::fs::create_dir_all(root.join(".cargo")).map_err(io_err)?;
    let files = [
        ("symdev.toml".to_string(), manifest(name)),
        ("Cargo.toml".into(), cargo_manifest(name, &sdk)),
        (".cargo/config.toml".into(), cargo_config(&sdk)),
        ("rust-toolchain.toml".into(), RustSdk::TOOLCHAIN_FILE.into()),
        ("src/main.rs".into(), RustSdk::HELLO_MAIN.into()),
    ];
    for (path, text) in files {
        std::fs::write(root.join(path), text).map_err(io_err)?;
    }
    Ok(root.to_path_buf())
}

fn manifest(name: &str) -> String {
    format!(
        "[package]\n\
         name = \"{name}\"\n\
         version = \"0.1.0\"\n\
         \n\
         [target]\n\
         device = \"nokia-e52\"\n\
         \n\
         [language]\n\
         name = \"rust\"\n\
         \n\
         [symbian]\n\
         uid3 = \"{}\"\n\
         capabilities = []\n\
         vendor = \"symdev\"\n\
         \n\
         [signing]\n\
         mode = \"self-signed\"\n",
        uid3_hex(name)
    )
}

/// A `staticlib` named after the package (`RustBuild` looks for `lib<name>.a`); the SDK
/// crates by absolute path; the same profile as the SDK workspace (size, one object,
/// no unwinder).
///
/// `symbian-std` and not `symbian-runtime`: the entry point is `#[symbian_std::main]`
/// (experiment 81), and the runtime underneath it — the panic handler, the heap and
/// the older `entry!` — comes with it, so the template names one crate for the road
/// and one (`symbian-core`) for the descriptors the escape hatch still needs.
fn cargo_manifest(name: &str, sdk: &RustSdk) -> String {
    format!(
        "[package]\n\
         name = \"{name}\"\n\
         version = \"0.1.0\"\n\
         edition = \"2024\"\n\
         # `src/main.rs` is a library: rustc never links, symdev does.\n\
         autobins = false\n\
         \n\
         [lib]\n\
         path = \"src/main.rs\"\n\
         crate-type = [\"staticlib\"]\n\
         \n\
         [dependencies]\n\
         symbian-core = {{ path = \"{}\" }}\n\
         symbian-std = {{ path = \"{}\" }}\n\
         \n\
         [profile.release]\n\
         opt-level = \"s\"\n\
         lto = true\n\
         codegen-units = 1\n\
         panic = \"abort\"\n\
         debug = false\n\
         \n\
         [profile.dev]\n\
         panic = \"abort\"\n",
        sdk.crate_dir("symbian-core").display(),
        sdk.crate_dir("symbian-std").display()
    )
}

/// What `symdev build` passes on the cargo command line, so a hand `cargo build` (or
/// `cargo clippy`) in the project does the same.
fn cargo_config(sdk: &RustSdk) -> String {
    format!(
        "# Kept in step with `symdev build` (RustBuild::cargo_args).\n\
         [build]\n\
         target = \"{}\"\n\
         target-dir = \"build/cargo\"\n\
         \n\
         [unstable]\n\
         build-std = [\"core\", \"alloc\"]\n\
         json-target-spec = true\n",
        sdk.target_spec().display()
    )
}

#[cfg(test)]
mod tests {
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
        let root = create_project(&dir, "hello", Template::Console, Lang::Rust).unwrap();
        let sdk = RustSdk::from_env().unwrap();
        let read = |p: &str| std::fs::read_to_string(root.join(p)).unwrap();
        assert!(read("symdev.toml").contains("name = \"rust\""));
        assert!(read("symdev.toml").contains("uid3 = \"0xef9f2cab\""));
        let cargo = read("Cargo.toml");
        assert!(cargo.contains("crate-type = [\"staticlib\"]"));
        assert!(cargo.contains(&sdk.crate_dir("symbian-core").display().to_string()));
        assert!(cargo.contains(&sdk.crate_dir("symbian-std").display().to_string()));
        assert!(read(".cargo/config.toml").contains(&sdk.target_spec().display().to_string()));
        assert_eq!(read("rust-toolchain.toml"), RustSdk::TOOLCHAIN_FILE);
        assert_eq!(read("src/main.rs"), RustSdk::HELLO_MAIN);
        assert!(!root.join("group").exists());
        assert!(!root.join("bld.inf").exists());
    }

    #[test]
    fn rust_gui_template_is_a_todo() {
        let dir = scratch();
        let err = create_project(&dir, "notes", Template::Gui, Lang::Rust).unwrap_err();
        assert!(err.to_string().starts_with("TODO:"), "{err}");
    }
}
