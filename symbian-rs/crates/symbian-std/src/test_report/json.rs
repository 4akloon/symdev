//! The report as JSON, written by appends alone: the file must not be why an example
//! links `core::fmt` (experiment 103). The shape is the module's; symdev's reader
//! (`crates/symdev-emulator/src/results.rs`) is the other half of it.
use alloc::string::String;
// What the fast `write!` falls back to for a piece not on its list; nothing here has one.
use core::fmt::Write as _;

use super::{Report, SCHEMA};
use crate::write;

/// The document for `report`, exactly as it goes into the file.
pub(super) fn document(report: &Report) -> String {
    let mut out = String::new();
    // Appending to a `String` cannot fail; the results are dropped for that reason.
    let _ = write!(out, "{{\"schema\":{},\"app\":\"", SCHEMA);
    escape_into(&mut out, &report.app);
    out.push_str("\",\"uid3\":\"0x");
    push_hex(&mut out, report.uid3, 8);
    let _ = write!(
        out,
        "\",\"passed\":{},\"failed\":{},\"cases\":[",
        report.passed(),
        report.failed()
    );
    for (i, case) in report.cases.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push_str("{\"name\":\"");
        escape_into(&mut out, &case.name);
        out.push_str(if case.ok {
            "\",\"ok\":true"
        } else {
            "\",\"ok\":false"
        });
        if !case.detail.is_empty() {
            out.push_str(",\"detail\":\"");
            escape_into(&mut out, &case.detail);
            out.push('"');
        }
        out.push('}');
    }
    out.push_str("]}");
    out
}

/// Appends `text` to `out` as the body of a JSON string.
///
/// Escapes what RFC 8259 requires — the quote, the backslash and everything below
/// `0x20` — and passes the rest through, since the file is UTF-8 and so is a Rust
/// `str`.
fn escape_into(out: &mut String, text: &str) {
    // Byte by byte, appending the runs between escapes whole: decoding and re-encoding
    // every character was 668 bytes of `examples/atomics`.
    let mut start = 0;
    for (i, byte) in text.bytes().enumerate() {
        let escape = match byte {
            b'"' => "\\\"",
            b'\\' => "\\\\",
            b'\n' => "\\n",
            b'\r' => "\\r",
            b'\t' => "\\t",
            0..0x20 => "\\u00",
            _ => continue,
        };
        // `i` is a character boundary: every byte escaped is ASCII, and an ASCII byte is
        // never part of a longer UTF-8 sequence. So `get` is always `Some` here.
        out.push_str(text.get(start..i).unwrap_or_default());
        out.push_str(escape);
        // `\u00` is the one escape that is followed by digits.
        if escape.len() == 4 {
            push_hex(out, u32::from(byte), 2);
        }
        start = i + 1;
    }
    out.push_str(text.get(start..).unwrap_or_default());
}

/// `value` in lowercase hex, zero-padded to at least `width` digits: `{:0width$x}`.
pub(super) fn push_hex(out: &mut String, value: u32, width: u32) {
    let digits = (8 - value.leading_zeros() / 4).max(width);
    for shift in (0..digits).rev() {
        // Nibbles beyond the value's own are zero, which is the padding.
        let nibble = value.checked_shr(shift * 4).unwrap_or(0) & 0xf;
        let digit = if nibble < 10 {
            b'0' + nibble as u8
        } else {
            b'a' - 10 + nibble as u8
        };
        out.push(char::from(digit));
    }
}
