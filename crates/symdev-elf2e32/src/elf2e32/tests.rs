//! Shared fixtures for `Elf2E32` encode/parse tests, split by experiment.
use super::*;

mod experiment_44;
mod experiment_49;
mod experiment_52;
mod experiment_54;
mod experiment_6;
mod fp2;
mod from_args;

fn hello_elf() -> ElfImage {
    let hex = include_str!("../testdata/hello.elf.hex");
    let hex: String = hex.chars().filter(|c| !c.is_whitespace()).collect();
    let bytes: Vec<u8> = (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
        .collect();
    ElfImage::parse(bytes).unwrap()
}

fn hello_ordinals() -> E32Ordinals {
    ordinals_table(include_str!("../testdata/hello_ordinals.txt"))
}

fn unhex(hex: &str) -> Vec<u8> {
    let hex: String = hex.chars().filter(|c| !c.is_whitespace()).collect();
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
        .collect()
}

fn ordinals_table(text: &str) -> E32Ordinals {
    let mut ordinals = E32Ordinals::default();
    for line in text.lines() {
        if line.starts_with('#') {
            continue;
        }
        let f: Vec<&str> = line.split_whitespace().collect();
        let ordinal = u32::from_str_radix(f[2].trim_start_matches("0x"), 16).unwrap();
        ordinals.insert(f[0], f[1], ordinal);
    }
    ordinals
}

fn args(tokens: &[&str]) -> Vec<String> {
    tokens.iter().map(|t| (*t).to_string()).collect()
}

fn experiment_6() -> Elf2E32 {
    Elf2E32::from_args(&args(&[
        "elf2e32",
        "--uid1=0x1000007a",
        "--uid3=0xe79e4cf9",
        "--capability=LocalServices+NetworkServices+ReadUserData+WriteUserData+UserEnvironment+Location",
        "--fpu=softvfp",
        "--targettype=EXE",
        "--output=hello.exe",
        "--elfinput=hello.elf",
        "--linkas=hello{000a0000}[e79e4cf9].exe",
        "--libpath=/sdk/epoc32/release/armv5/lib",
    ]))
    .unwrap()
}
