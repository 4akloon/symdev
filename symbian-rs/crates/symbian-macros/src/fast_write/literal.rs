//! The value of a literal token, from the text `Literal::to_string()` renders.
//!
//! Two jobs: the format string itself (a `"…"` or `r#"…"#`), and the literal
//! *arguments* rustc folds into the template at compile time — a string literal, or an
//! integer literal whose value fits its type (experiment 100). A fast `write!` has to
//! fold exactly the same ones, or its `write_str` calls would differ from `write!`'s.
//! Anything this module does not recognise is `None`, and the caller falls back to
//! `core::write!` rather than guessing.

/// The value of a string literal: `"…"` with its escapes, or a raw `r"…"`/`r#"…"#`.
/// Byte and C strings, and a literal with a suffix, are `None`.
pub fn string_value(source: &str) -> Option<String> {
    if let Some(raw) = source.strip_prefix('r') {
        let hashes = raw.len() - raw.trim_start_matches('#').len();
        let body = raw[hashes..].strip_prefix('"')?;
        let body = body.strip_suffix(&"#".repeat(hashes))?;
        return body.strip_suffix('"').map(String::from);
    }
    unescape(source.strip_prefix('"')?.strip_suffix('"')?)
}

/// The escapes of a Rust string literal (the Reference, "String literals").
fn unescape(body: &str) -> Option<String> {
    let mut out = String::with_capacity(body.len());
    let mut chars = body.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\r' {
            // A bare CR is not allowed in a string literal; CRLF reads as LF.
            chars.next_if_eq(&'\n')?;
            out.push('\n');
            continue;
        }
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next()? {
            'n' => out.push('\n'),
            'r' => out.push('\r'),
            't' => out.push('\t'),
            '\\' => out.push('\\'),
            '0' => out.push('\0'),
            '\'' => out.push('\''),
            '"' => out.push('"'),
            'x' => {
                let hex: String = [chars.next()?, chars.next()?].iter().collect();
                let value = u8::from_str_radix(&hex, 16).ok().filter(|v| *v <= 0x7f)?;
                out.push(char::from(value));
            }
            'u' => {
                if chars.next()? != '{' {
                    return None;
                }
                let mut hex = String::new();
                loop {
                    match chars.next()? {
                        '}' => break,
                        '_' => {}
                        d => hex.push(d),
                    }
                }
                out.push(char::from_u32(u32::from_str_radix(&hex, 16).ok()?)?);
            }
            // A line continuation: the newline and the whitespace after it vanish.
            '\n' | '\r' => {
                while chars
                    .next_if(|c| matches!(c, ' ' | '\t' | '\n' | '\r'))
                    .is_some()
                {}
            }
            _ => return None,
        }
    }
    Some(out)
}

/// The decimal text of an integer literal rustc would fold into the template: the
/// value must fit the literal's type, an unsuffixed literal being `i32`. `usize` and
/// `isize` are taken as 32 bits, the phone's width, so a value that only a 64-bit host
/// could hold is left as an argument (it is a compile error on the phone anyway).
pub fn folded_integer(source: &str) -> Option<String> {
    let first = source.chars().next()?;
    if !first.is_ascii_digit() {
        return None;
    }
    let (radix, digits) = match source.get(..2) {
        Some("0x") => (16, &source[2..]),
        Some("0o") => (8, &source[2..]),
        Some("0b") => (2, &source[2..]),
        _ => (10, source),
    };
    let end = digits
        .find(|c: char| !(c.is_digit(radix) || c == '_'))
        .unwrap_or(digits.len());
    let (number, suffix) = digits.split_at(end);
    let number: String = number.chars().filter(|c| *c != '_').collect();
    if number.is_empty() {
        return None;
    }
    let value = u128::from_str_radix(&number, radix).ok()?;
    let max: u128 = match suffix {
        "" | "i32" | "isize" => i32::MAX as u128,
        "u8" => u8::MAX.into(),
        "i8" => i8::MAX as u128,
        "u16" => u16::MAX.into(),
        "i16" => i16::MAX as u128,
        "u32" | "usize" => u32::MAX.into(),
        "u64" => u64::MAX.into(),
        "i64" => i64::MAX as u128,
        "u128" => u128::MAX,
        "i128" => i128::MAX as u128,
        _ => return None,
    };
    (value <= max).then(|| value.to_string())
}
