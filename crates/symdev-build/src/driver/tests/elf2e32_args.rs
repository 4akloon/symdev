use std::path::{Path, PathBuf};

use super::*;

#[test]
fn native_elf2e32_args_parse_as_the_recorded_job() {
    let mut d = fake();
    d.tools.elf2e32 = None;
    let args = d.elf2e32_args(
        "hello",
        Path::new("/p/build/hello.elf"),
        Path::new("/p/build/hello.exe"),
    );
    assert_eq!(args[0], "elf2e32");
    let job = symdev_elf2e32::Elf2E32::from_args(&args).unwrap();
    assert_eq!(job.uid3, 0xe79e4cf9);
    assert_eq!(job.output, PathBuf::from("/p/build/hello.exe"));
    assert_eq!(job.libpath, PathBuf::from("/sdk/epoc32/release/armv5/lib"));
    assert!(!job.uncompressed);
}

#[test]
fn elf2e32_args_empty_capabilities_omit_flag() {
    let d = fake();
    let args = d.elf2e32_args(
        "hello",
        Path::new("/proj/build/hello.elf"),
        Path::new("/proj/build/hello.exe"),
    );
    assert_eq!(
        args,
        s(&[
            "/gcc/elf2e32",
            "--uid1=0x1000007a",
            "--uid3=0xe79e4cf9",
            "--fpu=softvfp",
            "--targettype=EXE",
            "--output=/proj/build/hello.exe",
            "--elfinput=/proj/build/hello.elf",
            "--linkas=hello{000a0000}[e79e4cf9].exe",
            "--libpath=/sdk/epoc32/release/armv5/lib",
        ])
    );
    assert!(!args.iter().any(|a| a.contains("--capability")));
}

#[test]
fn elf2e32_args_join_manifest_capabilities_with_plus() {
    let mut d = fake();
    d.capabilities = vec!["ReadUserData".into(), "Location".into()];
    let args = d.elf2e32_args(
        "hello",
        Path::new("/proj/build/hello.elf"),
        Path::new("/proj/build/hello.exe"),
    );
    assert_eq!(
        args.iter()
            .find(|a| a.starts_with("--capability="))
            .map(String::as_str),
        Some("--capability=ReadUserData+Location")
    );
}
