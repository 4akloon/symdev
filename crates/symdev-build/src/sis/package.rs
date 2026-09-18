use std::path::{Path, PathBuf};
use std::process::Command;

use symdev_core::{Artifact, Error, LocalEnv, Package, PackageBackend, RemotePath, Result};

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

pub fn validate_password(password: &str) -> Result<()> {
    if password.len() < 4 {
        return Err(Error::Other(
            "SYMDEV_SIGN_PASSWORD must be at least 4 characters".into(),
        ));
    }
    Ok(())
}

pub fn write_pkg_file(
    dir: &Path,
    name: &str,
    uid3: u32,
    version: (u32, u32, u32),
    vendor: &str,
) -> Result<PathBuf> {
    let path = dir.join(format!("{name}.pkg"));
    std::fs::write(&path, crate::render_pkg(name, uid3, version, vendor))
        .map_err(|e| Error::Other(e.to_string()))?;
    Ok(path)
}

pub fn existing_signing_pair(
    cert: &Option<PathBuf>,
    key: &Option<PathBuf>,
) -> Option<(PathBuf, PathBuf)> {
    match (cert, key) {
        (Some(c), Some(k)) if c.is_file() && k.is_file() => Some((c.clone(), k.clone())),
        _ => None,
    }
}

pub struct SisPackage {
    pub env: LocalEnv,
    pub tools: SisTools,
    pub name: String,
    pub uid3: u32,
    pub version: (u32, u32, u32),
    pub vendor: String,
    pub password: String,
    pub cert: Option<PathBuf>,
    pub key: Option<PathBuf>,
}

impl SisPackage {
    fn run_tool(&self, args: &[String], cwd: &RemotePath) -> Result<()> {
        let mut cmd = Command::new(&args[0]);
        cmd.args(&args[1..]);
        let out = self.env.run_blocking(cmd, cwd)?;
        if out.status != 0 {
            return Err(Error::Other(
                String::from_utf8_lossy(&out.stderr).into_owned(),
            ));
        }
        Ok(())
    }
}

impl PackageBackend for SisPackage {
    fn package(&self, artifacts: &[Artifact]) -> Result<Package> {
        validate_password(&self.password)?;
        let artifact = match artifacts {
            [one] => one,
            _ => return Err(Error::Other("no E32 artifact".into())),
        };
        let expected = format!("{}.exe", self.name);
        let name_ok = artifact
            .path
            .file_name()
            .is_some_and(|n| n == expected.as_str());
        if !name_ok || !artifact.path.is_file() {
            return Err(Error::Other("E32 not found".into()));
        }
        let workdir = artifact
            .path
            .parent()
            .ok_or_else(|| Error::Other("E32 not found".into()))?;
        write_pkg_file(workdir, &self.name, self.uid3, self.version, &self.vendor)?;
        let cwd = RemotePath::new(workdir.display().to_string());
        let pkg = format!("{}.pkg", self.name);
        let sis = format!("{}.sis", self.name);
        let sisx = format!("{}.sisx", self.name);
        self.run_tool(&self.tools.makesis_args(&pkg, &sis), &cwd)?;
        let (cer, key) = match existing_signing_pair(&self.cert, &self.key) {
            Some((cer, key)) => (path_arg(&cer), path_arg(&key)),
            None => {
                let key_name = format!("{}.key", self.name);
                let cer_name = format!("{}.cer", self.name);
                self.run_tool(
                    &self
                        .tools
                        .makekeys_args(&self.password, &key_name, &cer_name),
                    &cwd,
                )?;
                (cer_name, key_name)
            }
        };
        self.run_tool(
            &self
                .tools
                .signsis_args(&sis, &sisx, &cer, &key, &self.password),
            &cwd,
        )?;
        Ok(Package {
            primary: workdir.join(&sisx),
            companions: vec![workdir.join(&sis)],
        })
    }
}

fn required(key: &str) -> Result<PathBuf> {
    match std::env::var(key) {
        Ok(v) if !v.is_empty() => Ok(PathBuf::from(v)),
        _ => Err(Error::Other(format!("missing toolchain: {key}"))),
    }
}

fn path_arg(path: &Path) -> String {
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
fn from_env_missing_epocroot_errors() {
    let _guard = ENV.lock().unwrap();
    // SAFETY: ENV serializes tests that mutate these keys.
    unsafe {
        std::env::remove_var("SYMDEV_EPOCROOT");
    }
    let err = match SisTools::from_env() {
        Err(e) => e,
        Ok(_) => panic!("expected missing SYMDEV_EPOCROOT"),
    };
    assert_eq!(err.to_string(), "missing toolchain: SYMDEV_EPOCROOT");
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

#[test]
fn validate_password_rejects_shorter_than_four_characters() {
    let err = validate_password("abc").unwrap_err();
    assert_eq!(
        err.to_string(),
        "SYMDEV_SIGN_PASSWORD must be at least 4 characters"
    );
    assert!(validate_password("").is_err());
    validate_password("abcd").unwrap();
}

#[test]
fn write_pkg_file_writes_recorded_pkg_next_to_exe() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_pkg_file(dir.path(), "hello", 0xe79e4cf9, (0, 1, 0), "symdev").unwrap();
    assert_eq!(path, dir.path().join("hello.pkg"));
    let s = std::fs::read_to_string(&path).unwrap();
    assert!(s.contains("\r\n"));
    assert_eq!(
        s.replace("\r\n", "\n"),
        "&EN\n#{\"hello\"},(0xe79e4cf9),0,1,0,TYPE=SA\n%{\"symdev\"}\n:\"symdev\"\n[0x102752AE], 0, 0, 0, {\"S60ProductID\"}\n\"hello.exe\"\t\t-\"!:\\sys\\bin\\hello.exe\"\n"
    );
}

#[cfg(test)]
fn fake_pkg() -> SisPackage {
    SisPackage {
        env: LocalEnv,
        tools: sdk_tools(),
        name: "hello".into(),
        uid3: 0xe79e4cf9,
        version: (0, 1, 0),
        vendor: "symdev".into(),
        password: "secret".into(),
        cert: None,
        key: None,
    }
}

#[test]
fn package_empty_artifacts_is_no_e32_artifact() {
    let err = fake_pkg().package(&[]).unwrap_err();
    assert_eq!(err.to_string(), "no E32 artifact");
}

#[test]
fn package_missing_e32_file_errors() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("hello.exe");
    let err = fake_pkg().package(&[Artifact { path }]).unwrap_err();
    assert_eq!(err.to_string(), "E32 not found");
}

#[test]
fn package_short_password_errors_before_tools() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("hello.exe");
    std::fs::write(&path, b"").unwrap();
    let mut pkg = fake_pkg();
    pkg.password = "abc".into();
    let err = pkg.package(&[Artifact { path }]).unwrap_err();
    assert_eq!(
        err.to_string(),
        "SYMDEV_SIGN_PASSWORD must be at least 4 characters"
    );
}

#[test]
fn existing_signing_pair_when_both_files_exist() {
    let dir = tempfile::tempdir().unwrap();
    let cert = dir.path().join("hello.cer");
    let key = dir.path().join("hello.key");
    std::fs::write(&cert, b"c").unwrap();
    std::fs::write(&key, b"k").unwrap();
    assert_eq!(
        existing_signing_pair(&Some(cert.clone()), &Some(key.clone())),
        Some((cert, key))
    );
}

#[test]
fn existing_signing_pair_none_when_missing() {
    let dir = tempfile::tempdir().unwrap();
    let cert = dir.path().join("hello.cer");
    let key = dir.path().join("hello.key");
    std::fs::write(&cert, b"c").unwrap();
    assert_eq!(existing_signing_pair(&Some(cert.clone()), &Some(key)), None);
    assert_eq!(existing_signing_pair(&None, &None), None);
    assert_eq!(existing_signing_pair(&Some(cert), &None), None);
}
