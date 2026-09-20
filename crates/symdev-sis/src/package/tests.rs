use std::path::PathBuf;
use std::sync::Mutex;

use sha1::{Digest, Sha1};

use super::*;
use crate::{SisUid, SisUnsigned};

mod encode;
mod signed;
mod tools;

static ENV: Mutex<()> = Mutex::new(());

fn sdk_tools() -> SisTools {
    SisTools {
        wine: PathBuf::from("/usr/bin/wine"),
        makesis: PathBuf::from("/sdk/epoc32/tools/makesis.exe"),
        signsis: PathBuf::from("/sdk/epoc32/tools/signsis.exe"),
    }
}

fn parse_hex(s: &str) -> Vec<u8> {
    let hex: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
        .collect()
}

fn hello_sis_golden() -> Vec<u8> {
    parse_hex(include_str!("../testdata/hello_sis.hex"))
}

fn hello_exe_bytes() -> Vec<u8> {
    parse_hex(include_str!("../testdata/hello_type30.hex"))[60..].to_vec()
}

fn hello_datetime() -> crate::SisDateTime {
    crate::SisDateTime::new(
        crate::SisDate::new(2026, 8, 17),
        crate::SisTime::new(15, 18, 24),
    )
}

fn hello_caps() -> Vec<String> {
    [
        "LocalServices",
        "NetworkServices",
        "ReadUserData",
        "WriteUserData",
        "UserEnvironment",
        "Location",
    ]
    .into_iter()
    .map(str::to_string)
    .collect()
}
