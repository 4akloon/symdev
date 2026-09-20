//! Tests for experiment 44 (EXE, `--uncompressed`, capability list).
use super::*;

fn experiment_44() -> Elf2E32 {
    let mut a = args(&[
        "elf2e32",
        "--uid1=0x1000007a",
        "--uid3=0xe79e4cf9",
        "--capability=LocalServices+NetworkServices+ReadUserData+WriteUserData+UserEnvironment+Location",
        "--fpu=softvfp",
        "--targettype=EXE",
        "--output=hello_u.exe",
        "--elfinput=hello.elf",
        "--linkas=hello{000a0000}[e79e4cf9].exe",
        "--libpath=/sdk/epoc32/release/armv5/lib",
    ]);
    a.push("--uncompressed".into());
    Elf2E32::from_args(&a).unwrap()
}

#[test]
fn experiment_44_encode_matches_uncompressed_golden() {
    let hex = include_str!("../../testdata/hello_uncompressed.exe.hex");
    let hex: String = hex.chars().filter(|c| !c.is_whitespace()).collect();
    let golden: Vec<u8> = (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
        .collect();
    let job = experiment_44();
    assert!(job.uncompressed);
    let bytes = job
        .encode_elf(
            &hello_elf(),
            &hello_ordinals(),
            E32Time(0x00e3_3986_c0a8_ae80),
        )
        .unwrap();
    assert_eq!(bytes, golden);
}

#[test]
fn experiment_44_tool_args_end_with_uncompressed() {
    let args = crate::Elf2E32Tool::new(Path::new("/elf2e32")).args(&experiment_44());
    assert_eq!(args.last().map(String::as_str), Some("--uncompressed"));
}
