//! The order in which a Rust project finds its Rust SDK (spec §12), each branch on its
//! own: `SYMDEV_RUST_SDK`, the checkout symdev was built from, the `rust-sdk` package
//! from a `file://` source in a temporary home. Nothing here reads the process's
//! environment or the real checkout.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

use symdev_manifest::Language;
use symdev_sdk::{
    ArchiveEntry, Host, Index, IndexPackage, Pins, ReproducibleTarGz, RustSdkPackage,
};

use super::super::Provision;

/// A temporary `SYMDEV_HOME`, cache and config whose one source, `local`, is a `file://`
/// directory that holds this symdev's `rust-sdk` package.
struct World {
    tmp: tempfile::TempDir,
}

impl World {
    fn new() -> World {
        let world = World {
            tmp: tempfile::tempdir().unwrap(),
        };
        let id = Pins::rust_sdk();
        let tree = world.path("tree");
        sdk_tree(&tree);
        let repo = world.path("repo");
        let url = format!("{}/sdk.tar.gz", id.relative_path().display());
        fs::create_dir_all(repo.join(&url).parent().unwrap()).unwrap();
        let (sha256, size) = ReproducibleTarGz::pack(&tree, &["."], &repo.join(&url)).unwrap();
        let mut index = Index::empty();
        let archives = vec![ArchiveEntry {
            host: Host::Any,
            url,
            sha256,
            size,
        }];
        let package = IndexPackage {
            id,
            license: "MIT".into(),
            source_code: None,
            depends: vec![],
            archives,
        };
        index.insert(package).unwrap();
        fs::write(repo.join("index.toml"), index.to_toml().unwrap()).unwrap();
        world.sources(&format!(
            "builtin = false\n\n[[source]]\nname = \"local\"\nurl = \"file://{}\"\n",
            repo.display()
        ));
        world
    }

    fn path(&self, name: &str) -> PathBuf {
        self.tmp.path().join(name)
    }

    fn sources(&self, text: &str) {
        fs::create_dir_all(self.path("config/symdev")).unwrap();
        fs::write(self.path("config/symdev/sources.toml"), text).unwrap();
    }

    /// `symdev` in this world, with `checkout` as the tree it was built from and `vars`
    /// on top of the world's directories.
    fn provision(
        &self,
        offline: bool,
        checkout: Option<&Path>,
        vars: &[(&str, &Path)],
    ) -> Provision {
        let mut map: BTreeMap<String, OsString> = BTreeMap::new();
        map.insert("SYMDEV_HOME".into(), self.path("home").into());
        map.insert("XDG_CACHE_HOME".into(), self.path("cache").into());
        map.insert("XDG_CONFIG_HOME".into(), self.path("config").into());
        for (key, value) in vars {
            map.insert(key.to_string(), value.into());
        }
        Provision::from_lookup(offline, checkout.map(Path::to_path_buf), move |key| {
            map.get(key).cloned()
        })
    }

    fn package_dir(&self) -> PathBuf {
        self.path("home").join(Pins::rust_sdk().relative_path())
    }
}

/// A directory with the files a Rust SDK must have.
fn sdk_tree(root: &Path) {
    for file in RustSdkPackage::REQUIRED {
        let path = root.join(file);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, b"{}").unwrap();
    }
}

fn canonical(path: &Path) -> PathBuf {
    path.canonicalize().unwrap()
}

#[test]
fn the_variable_wins_over_the_checkout() {
    let w = World::new();
    let (set, checkout) = (w.path("set"), w.path("checkout"));
    sdk_tree(&set);
    sdk_tree(&checkout);
    let p = w.provision(false, Some(&checkout), &[("SYMDEV_RUST_SDK", &set)]);
    assert_eq!(p.rust_sdk().unwrap().root(), canonical(&set));
    assert!(p.needed_rust_sdk().is_none());
}

#[test]
fn a_variable_that_is_no_sdk_names_itself_and_installs_nothing() {
    let w = World::new();
    let checkout = w.path("checkout");
    sdk_tree(&checkout);
    let stale = w.path("stale");
    let p = w.provision(false, Some(&checkout), &[("SYMDEV_RUST_SDK", &stale)]);
    let e = p.rust_sdk().unwrap_err().to_string();
    assert!(
        e.starts_with("SYMDEV_RUST_SDK is set, but Rust SDK not found at"),
        "{e}"
    );
    assert!(e.contains(&stale.display().to_string()), "{e}");
    assert!(!w.path("home").exists());
}

#[test]
fn the_checkout_is_used_while_it_is_an_sdk() {
    let w = World::new();
    let checkout = w.path("checkout");
    sdk_tree(&checkout);
    let p = w.provision(false, Some(&checkout), &[]);
    assert_eq!(p.rust_sdk().unwrap().root(), canonical(&checkout));
    assert!(p.needed_rust_sdk().is_none());
    assert!(!w.path("home").exists());
}

#[test]
fn without_a_checkout_the_package_is_installed() {
    let w = World::new();
    let p = w.provision(false, None, &[]);
    assert_eq!(p.needed_rust_sdk(), Some(Pins::rust_sdk()));
    assert_eq!(p.rust_sdk().unwrap().root(), canonical(&w.package_dir()));
    assert!(w.package_dir().join(".symdev-package.toml").is_file());
}

#[test]
fn a_checkout_that_is_gone_or_no_sdk_falls_back_to_the_package() {
    let w = World::new();
    let gone = w.path("gone");
    let p = w.provision(false, Some(&gone), &[]);
    assert_eq!(p.rust_sdk().unwrap().root(), canonical(&w.package_dir()));
    let empty = w.path("empty");
    fs::create_dir_all(&empty).unwrap();
    let p = w.provision(false, Some(&empty), &[]);
    assert_eq!(p.rust_sdk().unwrap().root(), canonical(&w.package_dir()));
}

/// Installed, the package needs no source: a broken `sources.toml` cannot stop a build.
#[test]
fn an_installed_package_reads_no_source() {
    let w = World::new();
    w.provision(false, None, &[]).rust_sdk().unwrap();
    w.sources("builtin = maybe\n");
    let p = w.provision(true, None, &[]);
    assert_eq!(p.rust_sdk().unwrap().root(), canonical(&w.package_dir()));
}

#[test]
fn offline_a_missing_package_names_the_install_command_and_the_variable() {
    let w = World::new();
    let e = w
        .provision(true, None, &[])
        .rust_sdk()
        .unwrap_err()
        .to_string();
    let id = Pins::rust_sdk();
    assert!(
        e.contains(&format!(
            "{id} is not installed and --offline forbids downloading it; run `symdev sdk \
             install {}`",
            id.shell_word()
        )),
        "{e}"
    );
    assert!(
        e.ends_with("or set SYMDEV_RUST_SDK to a symbian-rs directory"),
        "{e}"
    );
    assert!(!w.package_dir().exists());
}

#[test]
fn a_package_in_no_source_says_how_to_do_without_it() {
    let w = World::new();
    w.sources("builtin = false\n");
    let e = w
        .provision(false, None, &[])
        .rust_sdk()
        .unwrap_err()
        .to_string();
    assert!(e.contains("no package source is configured"), "{e}");
    assert!(
        e.ends_with("or set SYMDEV_RUST_SDK to a symbian-rs directory"),
        "{e}"
    );
}

#[test]
fn only_a_rust_project_needs_the_rust_sdk() {
    let w = World::new();
    let p = w.provision(false, None, &[]);
    let rust = p.needed(symdev_manifest::Device::NokiaE52, Language::Rust);
    assert_eq!(rust.first(), Some(&Pins::rust_sdk()));
    let std = p.needed(symdev_manifest::Device::NokiaE52, Language::RustStd);
    assert_eq!(std.first(), Some(&Pins::rust_sdk()));
    let cpp = p.needed(symdev_manifest::Device::NokiaE52, Language::Cpp);
    assert!(!cpp.contains(&Pins::rust_sdk()), "{cpp:?}");
}
