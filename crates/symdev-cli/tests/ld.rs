use std::path::{Path, PathBuf};
use std::process::Command;

fn symdev_ld(dir: &Path) -> PathBuf {
    let link = dir.join("symdev-ld");
    std::os::unix::fs::symlink(assert_cmd::cargo::cargo_bin("symdev"), &link).unwrap();
    link
}

fn fixture(name: &str) -> Vec<String> {
    let p = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src/ld/testdata")
        .join(name);
    std::fs::read_to_string(p)
        .unwrap()
        .lines()
        .map(String::from)
        .collect()
}

fn run(dir: &Path, envs: &[(&str, &Path)], args: &[String]) -> (bool, String) {
    let mut cmd = Command::new(symdev_ld(dir));
    cmd.args(args)
        .env_remove("CARGO_MANIFEST_DIR")
        .env_remove("CARGO_BIN_NAME")
        .env_remove("CARGO_TARGET_TMPDIR");
    for (k, v) in envs {
        cmd.env(k, v);
    }
    let out = cmd.output().unwrap();
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

#[test]
fn symdev_ld_outside_cargo_says_it_is_cargos_linker() {
    let dir = tempfile::tempdir().unwrap();
    let (ok, err) = run(dir.path(), &[], &fixture("release-bin.argv"));
    assert!(!ok);
    assert!(
        err.contains("CARGO_MANIFEST_DIR") && err.contains("cargo's linker"),
        "{err}"
    );
}

#[test]
fn symdev_ld_without_symdev_toml_names_the_directory() {
    let dir = tempfile::tempdir().unwrap();
    let project = dir.path().join("app");
    std::fs::create_dir(&project).unwrap();
    let (ok, err) = run(
        dir.path(),
        &[
            ("CARGO_MANIFEST_DIR", &project),
            ("CARGO_BIN_NAME", Path::new("app")),
        ],
        &fixture("release-bin.argv"),
    );
    assert!(!ok);
    assert!(
        err.contains(&project.display().to_string()) && err.contains("symdev.toml"),
        "{err}"
    );
}

#[test]
fn symdev_ld_refuses_an_unseen_argument_by_name() {
    let dir = tempfile::tempdir().unwrap();
    let mut args = fixture("release-bin.argv");
    args.push("--eh-frame-hdr".into());
    let (ok, err) = run(dir.path(), &[], &args);
    assert!(!ok && err.contains("`--eh-frame-hdr`"), "{err}");
}
