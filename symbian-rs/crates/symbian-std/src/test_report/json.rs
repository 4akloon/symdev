//! The report as JSON, written by appends alone: the file must not be why an example
//! links `core::fmt` (experiment 102). The shape is the module's; symdev's reader
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
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                out.push_str("\\u");
                push_hex(out, c as u32, 4);
            }
            c => out.push(c),
        }
    }
}

/// `value` in lowercase hex, zero-padded to at least `width` digits: `{:0width$x}`.
pub(super) fn push_hex(out: &mut String, value: u32, width: usize) {
    let digits = (8 - value.leading_zeros() as usize / 4).max(1);
    for _ in digits..width {
        out.push('0');
    }
    for shift in (0..digits).rev() {
        let nibble = (value >> (shift * 4)) & 0xf;
        out.push(char::from(b"0123456789abcdef"[nibble as usize]));
    }
}
