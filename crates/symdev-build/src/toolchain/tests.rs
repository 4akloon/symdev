use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

use symdev_sdk::{Gcce, PackageId, PlatformSdk};

use super::{Epocroot, Toolchain, ToolchainOverrides};

/// An installed `gcce;12.1.0` prefix with the files `Gcce::at` checks.
fn gcce(root: &Path) -> Gcce {
    fs::create_dir_all(root.join("bin")).unwrap();
    fs::write(root.join("bin/arm-none-symbianelf-g++"), b"").unwrap();
    fs::write(root.join("bin/arm-none-symbianelf-ld"), b"").unwrap();
    fs::create_dir_all(root.join("lib/gcc/arm-none-symbianelf/12.1.0")).unwrap();
    fs::create_dir_all(root.join("arm-none-symbianelf/lib")).unwrap();
    Gcce::at(
        root.to_path_buf(),
        &PackageId::parse("gcce;12.1.0").unwrap(),
    )
    .unwrap()
}

fn sdk(root: &Path) -> PlatformSdk {
    fs::create_dir_all(root.join("epoc32/include")).unwrap();
    PlatformSdk::at(
        root.to_path_buf(),
        &PackageId::parse("sdk;s60-3rd-fp2;1.1").unwrap(),
    )
    .unwrap()
}

/// Every override set, each to a file or directory that exists under `dir`.
fn all_overrides(dir: &Path) -> ToolchainOverrides {
    let made = |name: &str| {
        let path = dir.join(name);
        fs::write(&path, b"").unwrap();
        Some(path)
    };
    ToolchainOverrides {
        epocroot: Some(dir.to_path_buf()),
        gxx: made("g++"),
        ld: made("ld"),
        ar: made("ar"),
        elf2e32: made("elf2e32"),
        gcc_lib: made("gcc-lib"),
        gcc_target_lib: made("target-lib"),
    }
}

#[test]
fn every_set_override_wins_over_the_packages() {
    let tmp = tempfile::tempdir().unwrap();
    let o = all_overrides(tmp.path());
    let (g, s) = (gcce(&tmp.path().join("gcce")), sdk(&tmp.path().join("sdk")));
    let t = Toolchain::resolve(&o, Some(&g), Some(&s)).unwrap();
    assert_eq!(t.epocroot, tmp.path());
    assert_eq!(t.gxx, tmp.path().join("g++"));
    assert_eq!(t.ld, tmp.path().join("ld"));
    assert_eq!(t.ar().unwrap(), tmp.path().join("ar"));
    assert_eq!(t.elf2e32, Some(tmp.path().join("elf2e32")));
    assert_eq!(t.gcc_lib, tmp.path().join("gcc-lib"));
    assert_eq!(t.gcc_target_lib, tmp.path().join("target-lib"));
    assert!(!o.needs_gcce() && !o.needs_sdk());
}

#[test]
fn with_nothing_set_every_field_comes_from_the_packages() {
    let tmp = tempfile::tempdir().unwrap();
    let (g, s) = (gcce(&tmp.path().join("gcce")), sdk(&tmp.path().join("sdk")));
    let o = ToolchainOverrides::default();
    assert!(o.needs_gcce() && o.needs_sdk());
    let t = Toolchain::resolve(&o, Some(&g), Some(&s)).unwrap();
    assert_eq!(t.epocroot, tmp.path().join("sdk"));
    assert_eq!(t.gxx, g.gxx());
    assert_eq!(t.ld, g.ld());
    assert_eq!(
        t.ar().unwrap(),
        g.ld().with_file_name("arm-none-symbianelf-ar")
    );
    assert_eq!(
        t.elf2e32, None,
        "the native post-linker unless SYMDEV_ELF2E32 is set"
    );
    assert_eq!(t.gcc_lib, g.gcc_lib());
    assert_eq!(t.gcc_target_lib, g.gcc_target_lib());
}

#[test]
fn only_gxx_set_still_takes_the_linker_and_libraries_from_gcce() {
    let tmp = tempfile::tempdir().unwrap();
    let g = gcce(&tmp.path().join("gcce"));
    let gxx = tmp.path().join("my-g++");
    fs::write(&gxx, b"").unwrap();
    let o = ToolchainOverrides {
        epocroot: Some(tmp.path().to_path_buf()),
        gxx: Some(gxx.clone()),
        ..ToolchainOverrides::default()
    };
    assert!(
        o.needs_gcce(),
        "ld and the libraries still come from the package"
    );
    assert!(!o.needs_sdk());
    let t = Toolchain::resolve(&o, Some(&g), None).unwrap();
    assert_eq!(t.gxx, gxx);
    assert_eq!(t.ld, g.ld());
    assert_eq!(t.gcc_lib, g.gcc_lib());
    assert_eq!(t.gcc_target_lib, g.gcc_target_lib());
}

#[test]
fn an_override_that_does_not_exist_names_the_variable_and_the_path() {
    let tmp = tempfile::tempdir().unwrap();
    let mut o = all_overrides(tmp.path());
    o.gxx = Some(PathBuf::from("/nonexistent/symdev/g++"));
    let e = Toolchain::resolve(&o, None, None)
        .err()
        .unwrap()
        .to_string();
    assert!(e.contains("SYMDEV_GXX"), "{e}");
    assert!(e.contains("/nonexistent/symdev/g++"), "{e}");
}

#[test]
fn a_field_with_neither_override_nor_package_names_the_install_command() {
    let tmp = tempfile::tempdir().unwrap();
    let o = ToolchainOverrides {
        epocroot: Some(tmp.path().to_path_buf()),
        ..ToolchainOverrides::default()
    };
    let e = Toolchain::resolve(&o, None, None)
        .err()
        .unwrap()
        .to_string();
    assert!(e.contains("SYMDEV_GXX"), "{e}");
    assert!(e.contains("symdev sdk install 'gcce;12.1.0'"), "{e}");
}

#[test]
fn the_epocroot_comes_from_the_override_else_the_sdk_package() {
    let tmp = tempfile::tempdir().unwrap();
    let s = sdk(&tmp.path().join("sdk"));
    let set = ToolchainOverrides {
        epocroot: Some(tmp.path().to_path_buf()),
        ..ToolchainOverrides::default()
    };
    assert_eq!(
        Epocroot::resolve(&set, Some(&s)).unwrap().path(),
        tmp.path()
    );
    let unset = ToolchainOverrides::default();
    let from_package = Epocroot::resolve(&unset, Some(&s)).unwrap();
    assert_eq!(from_package.path(), tmp.path().join("sdk"));
    let e = Epocroot::resolve(&unset, None).unwrap_err().to_string();
    assert!(e.contains("SYMDEV_EPOCROOT"), "{e}");
    assert!(e.contains("symdev sdk install"), "{e}");
}

#[test]
fn reads_each_variable_and_treats_an_empty_one_as_unset() {
    let o = ToolchainOverrides::from_lookup(|key| match key {
        "SYMDEV_GXX" => Some(OsString::from("/x/g++")),
        "SYMDEV_LD" => Some(OsString::new()),
        "SYMDEV_EPOCROOT" => Some(OsString::from("/sdk")),
        "SYMDEV_AR" => Some(OsString::from("/x/ar")),
        "SYMDEV_ELF2E32" => Some(OsString::from("/x/elf2e32")),
        "SYMDEV_GCC_LIB" => Some(OsString::from("/x/lib")),
        "SYMDEV_GCC_TARGET_LIB" => Some(OsString::from("/x/target")),
        _ => None,
    });
    assert_eq!(o.gxx, Some(PathBuf::from("/x/g++")));
    assert_eq!(o.ld, None);
    assert_eq!(o.epocroot, Some(PathBuf::from("/sdk")));
    assert_eq!(o.ar, Some(PathBuf::from("/x/ar")));
    assert_eq!(o.elf2e32, Some(PathBuf::from("/x/elf2e32")));
    assert_eq!(o.gcc_lib, Some(PathBuf::from("/x/lib")));
    assert_eq!(o.gcc_target_lib, Some(PathBuf::from("/x/target")));
    assert!(o.needs_gcce(), "SYMDEV_LD is empty");
}
