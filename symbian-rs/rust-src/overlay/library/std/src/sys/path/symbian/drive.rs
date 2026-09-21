//! The drive-letter half of a Symbian path, as one pure function over bytes.
//!
//! It is a file of its own so that the **host** can test it: `symdev-build`'s
//! `std_src` tests `include!` this exact file and run the table on an ordinary
//! `cargo test --workspace`. Nothing else in the overlay can be tested off the phone,
//! because everything else in it calls euser.
//!
//! The grammar is the whole of it. A Symbian path names a drive with one ASCII letter
//! and a colon — `C:`, `E:`, `Z:` — and there is nothing else to parse: this platform
//! has no UNC share, no `\\?\` verbatim form and no device namespace, so the four
//! other `Prefix` variants can never occur here.

/// The drive letter `bytes` begins with, uppercased, or `None` when it begins with
/// something else.
///
/// Uppercased because the file server is case-insensitive about a drive — `RFs` reaches
/// the same volume for `c:\x` and `C:\x` — so the two must compare equal as
/// `Prefix::Disk`, and the comparison is on this byte.
pub(crate) const fn parse_drive(bytes: &[u8]) -> Option<u8> {
    match bytes {
        [drive, b':', ..] if drive.is_ascii_alphabetic() => Some(drive.to_ascii_uppercase()),
        _ => None,
    }
}
