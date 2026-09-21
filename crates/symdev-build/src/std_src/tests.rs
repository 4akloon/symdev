//! Host tests for the one piece of the `std` overlay that can be tested off the phone.
//!
//! `sys/path/symbian/drive.rs` is pure: bytes in, an optional drive letter out, no
//! euser and no `std` internals. So it is compiled straight out of the overlay by
//! `#[path]` — the file that is copied into the patched standard library is the file
//! these cases run against, not a transcription of it — and `cargo test --workspace`
//! covers the prefix grammar that `Path::is_absolute`, `parent`, `join` and
//! `components` all rest on.
#[path = "../../../../symbian-rs/rust-src/overlay/library/std/src/sys/path/symbian/drive.rs"]
mod drive;

use drive::parse_drive;

#[test]
fn a_letter_and_a_colon_is_a_drive() {
    assert_eq!(parse_drive(b"C:"), Some(b'C'));
    assert_eq!(parse_drive(b"E:\\symdev\\x.txt"), Some(b'E'));
    assert_eq!(parse_drive(b"Z:\\sys\\bin"), Some(b'Z'));
}

#[test]
fn the_letter_is_uppercased_because_the_file_server_is_case_insensitive() {
    assert_eq!(parse_drive(b"c:\\x"), parse_drive(b"C:\\x"));
    assert_eq!(parse_drive(b"e:"), Some(b'E'));
}

#[test]
fn anything_that_is_not_a_letter_and_a_colon_is_not_a_drive() {
    assert_eq!(parse_drive(b""), None);
    assert_eq!(parse_drive(b"C"), None);
    assert_eq!(parse_drive(b":"), None);
    assert_eq!(parse_drive(b"1:"), None);
    assert_eq!(parse_drive(b"\\symdev"), None);
    assert_eq!(parse_drive(b"symdev\\x"), None);
    // Two letters is not a drive: Symbian names a volume with exactly one.
    assert_eq!(parse_drive(b"CD:"), None);
}

#[test]
fn there_is_no_unc_and_no_verbatim_form_to_recognise() {
    // On Windows these are `UNC` and `Verbatim*` prefixes. Symbian OS 9.3 has neither,
    // so they are an ordinary rooted path with empty leading components.
    assert_eq!(parse_drive(b"\\\\server\\share"), None);
    assert_eq!(parse_drive(b"\\\\?\\C:\\x"), None);
}
