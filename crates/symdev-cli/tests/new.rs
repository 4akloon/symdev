use predicates::prelude::*;

mod common;
use common::{HELLO, bin, write_toml};

#[test]
fn new_creates_hello_tree() {
    let dir = tempfile::tempdir().unwrap();
    let hello = dir.path().join("hello");
    bin()
        .current_dir(&dir)
        .args(["new", "hello", "--target", "nokia-e52"])
        .assert()
        .success()
        .code(0)
        .stdout(predicate::eq(format!("{}\n", hello.display())));

    assert!(hello.join("symdev.toml").is_file());
    assert!(hello.join("group/bld.inf").is_file());
    assert!(hello.join("group/hello.mmp").is_file());
    assert!(hello.join("src/hello.cpp").is_file());
    assert!(hello.join("src/hello.h").is_file());

    let toml = std::fs::read_to_string(hello.join("symdev.toml")).unwrap();
    assert!(toml.contains("uid3 = \"0xef9f2cab\""));
    assert_eq!(
        std::fs::read(hello.join("src/hello.cpp")).unwrap(),
        include_bytes!("../templates/hello.cpp")
    );
}

#[test]
fn new_existing_dir_errors() {
    let dir = tempfile::tempdir().unwrap();
    bin()
        .current_dir(&dir)
        .args(["new", "hello", "--target", "nokia-e52"])
        .assert()
        .success();

    let hello = dir.path().join("hello");
    let snapshot = [
        std::fs::read(hello.join("symdev.toml")).unwrap(),
        std::fs::read(hello.join("group/bld.inf")).unwrap(),
        std::fs::read(hello.join("group/hello.mmp")).unwrap(),
        std::fs::read(hello.join("src/hello.cpp")).unwrap(),
        std::fs::read(hello.join("src/hello.h")).unwrap(),
    ];

    bin()
        .current_dir(&dir)
        .args(["new", "hello", "--target", "nokia-e52"])
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains(
            "error: directory `hello` already exists",
        ));

    assert_eq!(
        std::fs::read(hello.join("symdev.toml")).unwrap(),
        snapshot[0]
    );
    assert_eq!(
        std::fs::read(hello.join("group/bld.inf")).unwrap(),
        snapshot[1]
    );
    assert_eq!(
        std::fs::read(hello.join("group/hello.mmp")).unwrap(),
        snapshot[2]
    );
    assert_eq!(
        std::fs::read(hello.join("src/hello.cpp")).unwrap(),
        snapshot[3]
    );
    assert_eq!(
        std::fs::read(hello.join("src/hello.h")).unwrap(),
        snapshot[4]
    );
}

#[test]
fn new_rejects_invalid_name() {
    bin()
        .args(["new", "1bad", "--target", "nokia-e52"])
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("not implemented").not());
}

#[test]
fn new_rejects_java() {
    bin()
        .args(["new", "hello", "--target", "nokia-e52", "--lang", "java"])
        .assert()
        .failure()
        .code(2);
}

#[test]
fn new_does_not_read_toml() {
    let dir = tempfile::tempdir().unwrap();
    write_toml(&dir, &HELLO.replace(r#"name = "cpp""#, r#"name = "java""#));
    bin()
        .current_dir(&dir)
        .args(["new", "hello", "--target", "nokia-e52"])
        .assert()
        .success()
        .stderr(predicate::str::contains("invalid manifest").not());
    assert!(dir.path().join("hello").is_dir());
}
