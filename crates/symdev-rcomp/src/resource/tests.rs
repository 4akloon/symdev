use super::*;
use crate::RscUid;

fn parse_hex(s: &str) -> Vec<u8> {
    let hex: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
        .collect()
}

fn driveinfo_rsc() -> Vec<u8> {
    parse_hex(include_str!("../testdata/driveinfo_reg.rsc.hex"))
}

fn filebrowse_rsc() -> Vec<u8> {
    parse_hex(include_str!("../testdata/filebrowseapp_reg.rsc.hex"))
}

#[test]
fn driveinfo_rsc_uid_bytes_match_experiment_41() {
    let golden = driveinfo_rsc();
    assert_eq!(golden.len(), 74);
    let want = [
        0x6b, 0x4a, 0x1f, 0x10, 0x21, 0x80, 0x1f, 0x10, 0xf4, 0x01, 0x00, 0xa0, 0xb4, 0x0c, 0xc8,
        0xf0,
    ];
    assert_eq!(RscUid::new(0x101f_8021, 0xa000_01f4).bytes(), want);
    assert_eq!(&golden[..16], &want);
}

#[test]
fn driveinfo_rsc_uid_checked_matches_experiment_41() {
    assert_eq!(
        RscUid::new(0x101f_8021, 0xa000_01f4).crc().checked(),
        0xf0c8_0cb4
    );
}

#[test]
fn filebrowse_rsc_uid_bytes_match_experiment_41() {
    let golden = filebrowse_rsc();
    assert_eq!(golden.len(), 109);
    let want = [
        0x6b, 0x4a, 0x1f, 0x10, 0x21, 0x80, 0x1f, 0x10, 0xa6, 0x00, 0x00, 0xe8, 0x69, 0x64, 0x35,
        0x0a,
    ];
    assert_eq!(RscUid::new(0x101f_8021, 0xe800_00a6).bytes(), want);
    assert_eq!(&golden[..16], &want);
}

#[test]
fn rsc_uid1_is_recorded_unicode_resource_file() {
    assert_eq!(RscUid::UID1, 0x101f_4a6b);
    assert_eq!(RscUid::REGISTRATION_UID2, 0x101f_8021);
}

#[test]
fn hello_registration_rsc_matches_experiment_43() {
    let golden = parse_hex(include_str!("../testdata/hello_reg.rsc.hex"));
    assert_eq!(golden.len(), 67);
    let rsc = Rsc::registration(0xe79e_4cf9, "hello").unwrap();
    assert_eq!(rsc.bytes().unwrap(), golden);
}

#[test]
fn registration_rsc_rejects_empty_app_file() {
    match Rsc::registration(0xe79e_4cf9, "") {
        Err(err) => assert!(err.to_string().contains("app_file empty")),
        Ok(_) => panic!("expected app_file empty"),
    }
}

fn driveinfo_reg() -> RscAppRegistration {
    RscAppRegistration::new(
        RscLtext16::new("DriveInfoApp").unwrap(),
        RscLtext16::new("").unwrap(),
        1,
    )
}

fn filebrowse_reg() -> RscAppRegistration {
    RscAppRegistration::new(
        RscLtext16::new("filebrowseapp").unwrap(),
        RscLtext16::new("\\resource\\apps\\filebrowseapp_loc").unwrap(),
        1,
    )
}

#[test]
fn driveinfo_rsc_bytes_match_experiment_41() {
    let rsc = Rsc::new(
        RscUid::new(0x101f_8021, 0xa000_01f4),
        vec![driveinfo_reg().data()],
    );
    assert_eq!(rsc.bytes().unwrap(), driveinfo_rsc());
}

#[test]
fn filebrowse_rsc_bytes_match_experiment_41() {
    let rsc = Rsc::new(
        RscUid::new(0x101f_8021, 0xe800_00a6),
        vec![filebrowse_reg().data()],
    );
    assert_eq!(rsc.bytes().unwrap(), filebrowse_rsc());
}

#[test]
fn rsc_ltext16_empty_is_a_zero_length_byte() {
    assert_eq!(RscLtext16::empty().bytes(), [0]);
}

#[test]
fn rsc_ltext16_driveinfo_is_length_pad_and_utf16le() {
    let bytes = RscLtext16::new("DriveInfoApp").unwrap().bytes();
    assert_eq!(bytes[0], 12);
    // The alignment pad is 0xAB and is dropped again when the resource is packed
    // (rcomp-spec.md §3.6), so the `.rsc` goldens are unaffected.
    assert_eq!(bytes[1], 0xab);
    assert_eq!(
        &bytes[2..],
        "DriveInfoApp"
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect::<Vec<_>>()
    );
}

#[test]
fn driveinfo_uncompressed_is_59_bytes_experiment_42() {
    let resource = driveinfo_reg().data();
    assert_eq!(resource.len(), 59);
    assert_eq!(resource.len() as u16, 59);
    assert_eq!(&driveinfo_rsc()[16..20], [0, 59, 0, 1]);
    assert_eq!(&driveinfo_rsc()[70..], [0x14, 0, 0x46, 0]);
}

#[test]
fn filebrowse_uncompressed_is_126_bytes_experiment_42() {
    let resource = filebrowse_reg().data();
    assert_eq!(resource.len(), 126);
    assert_eq!(&filebrowse_rsc()[16..20], [0, 126, 0, 1]);
    assert_eq!(&filebrowse_rsc()[105..], [0x14, 0, 0x69, 0]);
}

#[test]
fn rsc_ltext16_rejects_more_than_255_utf16_units() {
    match RscLtext16::new("a".repeat(256)) {
        Err(err) => assert!(err.to_string().contains("LText16 longer than 255")),
        Ok(_) => panic!("expected LText16 longer than 255"),
    }
}
