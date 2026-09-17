use std::path::PathBuf;

use symdev_core::{Error, Result};

pub struct SisTools {
    pub wine: PathBuf,
    pub makesis: PathBuf,
    pub signsis: PathBuf,
    pub makekeys: PathBuf,
}

impl SisTools {
    pub fn from_env() -> Result<Self> {
        let epocroot = required("SYMDEV_EPOCROOT")?;
        let wine = match std::env::var("SYMDEV_WINE") {
            Ok(v) if !v.is_empty() => PathBuf::from(v),
            _ => PathBuf::from("/usr/bin/wine"),
        };
        let tools = epocroot.join("epoc32/tools");
        Ok(Self {
            wine,
            makesis: tools.join("makesis.exe"),
            signsis: tools.join("signsis.exe"),
            makekeys: tools.join("makekeys.exe"),
        })
    }

    pub fn makesis_args(&self, pkg: &str, sis: &str) -> Vec<String> {
        vec![
            path_arg(&self.wine),
            path_arg(&self.makesis),
            "-v".into(),
            pkg.into(),
            sis.into(),
        ]
    }

    pub fn makekeys_args(&self, password: &str, key: &str, cer: &str) -> Vec<String> {
        vec![
            path_arg(&self.wine),
            path_arg(&self.makekeys),
            "-cert".into(),
            "-expdays".into(),
            "3650".into(),
            "-password".into(),
            password.into(),
            "-len".into(),
            "2048".into(),
            "-dname".into(),
            dname().into(),
            key.into(),
            cer.into(),
        ]
    }

    pub fn signsis_args(
        &self,
        sis: &str,
        sisx: &str,
        cer: &str,
        key: &str,
        password: &str,
    ) -> Vec<String> {
        vec![
            path_arg(&self.wine),
            path_arg(&self.signsis),
            sis.into(),
            sisx.into(),
            cer.into(),
            key.into(),
            password.into(),
        ]
    }
}

pub fn dname() -> &'static str {
    "CN=Joe Bloggs OU=Development O=Acme Ltd C=GB EM=noone@nowhere.com"
}

fn required(key: &str) -> Result<PathBuf> {
    match std::env::var(key) {
        Ok(v) if !v.is_empty() => Ok(PathBuf::from(v)),
        _ => Err(Error::Other(format!("missing toolchain: {key}"))),
    }
}

fn path_arg(path: &std::path::Path) -> String {
    path.display().to_string()
}

#[cfg(test)]
use std::sync::Mutex;

#[cfg(test)]
static ENV: Mutex<()> = Mutex::new(());

#[cfg(test)]
fn sdk_tools() -> SisTools {
    SisTools {
        wine: PathBuf::from("/usr/bin/wine"),
        makesis: PathBuf::from("/sdk/epoc32/tools/makesis.exe"),
        signsis: PathBuf::from("/sdk/epoc32/tools/signsis.exe"),
        makekeys: PathBuf::from("/sdk/epoc32/tools/makekeys.exe"),
    }
}

#[test]
fn from_env_uses_sdk_fixture_and_default_wine() {
    let _guard = ENV.lock().unwrap();
    // SAFETY: ENV serializes tests that mutate these keys.
    unsafe {
        std::env::set_var("SYMDEV_EPOCROOT", "/sdk");
        std::env::remove_var("SYMDEV_WINE");
    }
    let tools = SisTools::from_env().unwrap();
    assert_eq!(tools.wine, PathBuf::from("/usr/bin/wine"));
    assert_eq!(
        tools.makesis,
        PathBuf::from("/sdk/epoc32/tools/makesis.exe")
    );
    assert_eq!(
        tools.signsis,
        PathBuf::from("/sdk/epoc32/tools/signsis.exe")
    );
    assert_eq!(
        tools.makekeys,
        PathBuf::from("/sdk/epoc32/tools/makekeys.exe")
    );
}

#[test]
fn makesis_args_match_experiment_7() {
    assert_eq!(
        sdk_tools().makesis_args("hello.pkg", "hello.sis"),
        [
            "/usr/bin/wine",
            "/sdk/epoc32/tools/makesis.exe",
            "-v",
            "hello.pkg",
            "hello.sis",
        ]
    );
}

#[test]
fn makekeys_args_match_experiment_8() {
    assert_eq!(
        sdk_tools().makekeys_args("secret", "hello.key", "hello.cer"),
        [
            "/usr/bin/wine",
            "/sdk/epoc32/tools/makekeys.exe",
            "-cert",
            "-expdays",
            "3650",
            "-password",
            "secret",
            "-len",
            "2048",
            "-dname",
            "CN=Joe Bloggs OU=Development O=Acme Ltd C=GB EM=noone@nowhere.com",
            "hello.key",
            "hello.cer",
        ]
    );
}

#[test]
fn signsis_args_match_experiment_8() {
    assert_eq!(
        sdk_tools().signsis_args(
            "hello.sis",
            "hello.sisx",
            "hello.cer",
            "hello.key",
            "secret",
        ),
        [
            "/usr/bin/wine",
            "/sdk/epoc32/tools/signsis.exe",
            "hello.sis",
            "hello.sisx",
            "hello.cer",
            "hello.key",
            "secret",
        ]
    );
}

#[test]
fn dname_is_makekeys_example_usage() {
    assert_eq!(
        dname(),
        "CN=Joe Bloggs OU=Development O=Acme Ltd C=GB EM=noone@nowhere.com"
    );
}
