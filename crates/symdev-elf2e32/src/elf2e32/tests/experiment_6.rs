//! Tests for experiment 6 (`hello.exe`, EXE UID and encode goldens).
use super::*;

fn parse_hex(s: &str) -> Vec<u8> {
    let hex: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
        .collect()
}

fn hello_exe() -> Vec<u8> {
    parse_hex(include_str!("../../testdata/hello.exe.hex"))
}

#[test]
fn hello_exe_uid_bytes_match_experiment_6() {
    let golden = hello_exe();
    assert_eq!(golden.len(), 3588);
    let want = [
        0x7a, 0x00, 0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0xf9, 0x4c, 0x9e, 0xe7, 0xb0, 0x08, 0x32,
        0xc1,
    ];
    assert_eq!(E32Uid::for_exe(0x1000_007a, 0xe79e_4cf9).bytes(), want);
    assert_eq!(&golden[..16], &want);
}

#[test]
fn hello_exe_uid_checked_matches_experiment_6() {
    assert_eq!(
        E32Uid::for_exe(0x1000_007a, 0xe79e_4cf9).crc().checked(),
        0xc132_08b0
    );
}

#[test]
fn experiment_6_job_uid_matches_hello_exe_prefix() {
    let golden = hello_exe();
    assert_eq!(experiment_6().uid().bytes(), golden[..16]);
}

#[test]
fn experiment_6_encode_matches_frozen_hello_exe() {
    // Frozen experiment-6 hello.exe header time.
    let bytes = experiment_6()
        .encode_elf(
            &hello_elf(),
            &hello_ordinals(),
            E32Time(0x00e3_3963_208d_5e00),
        )
        .unwrap();
    assert_eq!(bytes.len(), 3588);
    assert_eq!(bytes, hello_exe());
}
