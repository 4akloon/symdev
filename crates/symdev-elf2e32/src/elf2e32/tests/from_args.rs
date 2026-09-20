//! Tests for `Elf2E32::from_args` argument parsing.
use super::*;

#[test]
fn from_args_match_experiment_6() {
    let job = experiment_6();
    assert_eq!(job.uid1, 0x1000_007a);
    assert_eq!(job.uid3, 0xe79e_4cf9);
    assert_eq!(
        job.capability.as_deref(),
        Some("LocalServices+NetworkServices+ReadUserData+WriteUserData+UserEnvironment+Location")
    );
    assert_eq!(job.fpu, "softvfp");
    assert_eq!(job.targettype, "EXE");
    assert_eq!(job.output, PathBuf::from("hello.exe"));
    assert_eq!(job.elfinput, PathBuf::from("hello.elf"));
    assert_eq!(job.linkas, "hello{000a0000}[e79e4cf9].exe");
    assert_eq!(job.libpath, PathBuf::from("/sdk/epoc32/release/armv5/lib"));
}

#[test]
fn from_args_omits_empty_capability() {
    let job = Elf2E32::from_args(&args(&[
        "elf2e32",
        "--uid1=0x1000007a",
        "--uid3=0xe79e4cf9",
        "--fpu=softvfp",
        "--targettype=EXE",
        "--output=hello.exe",
        "--elfinput=hello.elf",
        "--linkas=hello{000a0000}[e79e4cf9].exe",
        "--libpath=/sdk/epoc32/release/armv5/lib",
    ]))
    .unwrap();
    assert_eq!(job.capability, None);
}
