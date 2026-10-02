use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::PathBuf;

use symdev_sdk::{Auth, SourceSpec};

use super::Provision;

fn provision(vars: &[(&str, &str)]) -> Provision {
    let vars: BTreeMap<String, OsString> = vars
        .iter()
        .map(|(k, v)| (k.to_string(), OsString::from(v)))
        .collect();
    Provision::from_lookup(false, move |key| vars.get(key).cloned())
}

#[test]
fn defaults_follow_xdg_under_home() {
    let p = provision(&[("HOME", "/home/u")]);
    assert_eq!(
        p.home_dir().unwrap(),
        PathBuf::from("/home/u/.local/share/symdev")
    );
    assert_eq!(
        p.cache_dir().unwrap(),
        PathBuf::from("/home/u/.cache/symdev/downloads")
    );
    assert_eq!(
        p.sources_file().unwrap(),
        PathBuf::from("/home/u/.config/symdev/sources.toml")
    );
}

#[test]
fn xdg_variables_and_symdev_home_move_them() {
    let p = provision(&[
        ("HOME", "/home/u"),
        ("XDG_DATA_HOME", "/data"),
        ("XDG_CACHE_HOME", "/cache"),
        ("XDG_CONFIG_HOME", "/config"),
    ]);
    assert_eq!(p.home_dir().unwrap(), PathBuf::from("/data/symdev"));
    assert_eq!(
        p.cache_dir().unwrap(),
        PathBuf::from("/cache/symdev/downloads")
    );
    assert_eq!(
        p.sources_file().unwrap(),
        PathBuf::from("/config/symdev/sources.toml")
    );
    let p = provision(&[("SYMDEV_HOME", "/opt/symdev"), ("XDG_DATA_HOME", "/data")]);
    assert_eq!(p.home_dir().unwrap(), PathBuf::from("/opt/symdev"));
}

#[test]
fn a_relative_xdg_variable_is_ignored_as_the_spec_says() {
    let p = provision(&[("HOME", "/home/u"), ("XDG_DATA_HOME", "data")]);
    assert_eq!(
        p.home_dir().unwrap(),
        PathBuf::from("/home/u/.local/share/symdev")
    );
}

#[test]
fn without_home_the_error_names_the_variables_to_set() {
    let e = provision(&[]).home_dir().unwrap_err().to_string();
    assert!(e.contains("SYMDEV_HOME") && e.contains("HOME"), "{e}");
    let e = provision(&[("SYMDEV_HOME", "relative")])
        .home_dir()
        .unwrap_err()
        .to_string();
    assert!(e.contains("SYMDEV_HOME") && e.contains("absolute"), "{e}");
}

#[test]
fn reads_the_keys_of_s3_sources_only() {
    let p = provision(&[
        ("SYMDEV_SOURCE_PRIVATE_ACCESS_KEY_ID", "AKID"),
        ("SYMDEV_SOURCE_PRIVATE_SECRET_ACCESS_KEY", "secret"),
        ("SYMDEV_SOURCE_MIRROR_ACCESS_KEY_ID", "unused"),
        ("SYMDEV_SOURCE_MIRROR_SECRET_ACCESS_KEY", "unused"),
    ]);
    let sources = [
        SourceSpec::new("private", "https://example.com/p/", Auth::S3).unwrap(),
        SourceSpec::new("mirror", "https://example.com/m/", Auth::None).unwrap(),
        SourceSpec::new("other", "https://example.com/o/", Auth::S3).unwrap(),
    ];
    let keys = p.keys(&sources).unwrap();
    assert_eq!(keys.keys().collect::<Vec<_>>(), ["private"]);
    assert_eq!(keys["private"].access_key_id, "AKID");
    assert_eq!(keys["private"].secret_access_key, "secret");
}

#[test]
fn half_a_key_pair_names_the_missing_variable() {
    let p = provision(&[("SYMDEV_SOURCE_PRIVATE_ACCESS_KEY_ID", "AKID")]);
    let sources = [SourceSpec::new("private", "https://example.com/p/", Auth::S3).unwrap()];
    let e = p.keys(&sources).unwrap_err().to_string();
    assert!(e.contains("SYMDEV_SOURCE_PRIVATE_SECRET_ACCESS_KEY"), "{e}");
}
