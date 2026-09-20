use super::*;

mod native;
mod self_signed;
mod text;
mod validation;

fn fake_pkg() -> SisPackage {
    SisPackage {
        name: "hello".into(),
        app: "hello".into(),
        uid3: 0xe79e4cf9,
        version: (0, 1, 0),
        vendor: "symdev".into(),
        capabilities: Vec::new(),
        password: "secret".into(),
        cert: None,
        key: None,
        subject: None,
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
    parse_hex(include_str!(
        "../../../symdev-sis/src/testdata/hello_sis.hex"
    ))
}

fn hello_exe_bytes() -> Vec<u8> {
    parse_hex(include_str!(
        "../../../symdev-sis/src/testdata/hello_type30.hex"
    ))[60..]
        .to_vec()
}

fn hello_datetime() -> symdev_sis::SisDateTime {
    symdev_sis::SisDateTime::new(
        symdev_sis::SisDate::new(2026, 8, 17),
        symdev_sis::SisTime::new(15, 18, 24),
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
