//! `Table`: one locales file.
//!
//! The syntax is a strict subset of TOML — `key = "value"`, one per line, `#` comments
//! and blank lines — so that any TOML-aware editor highlights it, and small enough that
//! this crate can read it with no parser dependency. Everything outside the subset is an
//! error naming the file and the line, rather than something silently read differently
//! from what a TOML reader would make of it.
use std::collections::BTreeMap;

use crate::Error;

/// The keys that translate `[ui] caption` / `short_caption` rather than being strings.
pub const RESERVED: [&str; 2] = ["caption", "short_caption"];

/// One file's entries, keys in byte order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Table {
    pub entries: BTreeMap<String, String>,
}

impl Table {
    /// Reads `text`; `file` is only for the messages.
    pub fn parse(file: &str, text: &str) -> Result<Table, Error> {
        let mut entries = BTreeMap::new();
        for (n, raw) in text.lines().enumerate() {
            let at = || format!("{file}:{}", n + 1);
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let (key, value) = line.split_once('=').ok_or_else(|| {
                Error(format!(
                    "{}: expected `key = \"text\"`, found `{line}`",
                    at()
                ))
            })?;
            let key = key.trim();
            if !is_key(key) {
                return Err(Error(format!(
                    "{}: `{key}` is not a key: use lower-case letters, digits and `_`, \
                     starting with a letter",
                    at()
                )));
            }
            let value = string(value.trim()).map_err(|why| Error(format!("{}: {why}", at())))?;
            if entries.insert(key.to_string(), value).is_some() {
                return Err(Error(format!("{}: `{key}` is given twice", at())));
            }
        }
        Ok(Table { entries })
    }

    /// The translated caption, if this file gives one.
    pub fn caption(&self) -> Option<&str> {
        self.entries.get("caption").map(String::as_str)
    }

    /// The translated short caption, if this file gives one.
    pub fn short_caption(&self) -> Option<&str> {
        self.entries.get("short_caption").map(String::as_str)
    }

    /// The string keys: every key except the reserved caption pair.
    pub fn strings(&self) -> impl Iterator<Item = &str> {
        self.entries
            .keys()
            .map(String::as_str)
            .filter(|k| !RESERVED.contains(k))
    }
}

fn is_key(key: &str) -> bool {
    let mut chars = key.chars();
    chars.next().is_some_and(|c| c.is_ascii_lowercase())
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

/// A TOML basic string: `"…"` with `\"`, `\\`, `\n`, `\t` and `\uXXXX`. U+0000 is
/// refused because `rcomp`'s `<0x0000>` literal is broken in this build
/// (`rcomp-spec.md` §3), so it could not reach the compiled file intact.
fn string(text: &str) -> Result<String, String> {
    let body = text
        .strip_prefix('"')
        .and_then(|t| t.strip_suffix('"'))
        .filter(|_| text.len() >= 2)
        .ok_or_else(|| format!("expected a quoted string, found `{text}`"))?;
    let mut out = String::new();
    let mut chars = body.chars();
    while let Some(c) = chars.next() {
        let c = match c {
            '"' => return Err("an unescaped `\"` inside the string".into()),
            '\\' => match chars.next() {
                Some('"') => '"',
                Some('\\') => '\\',
                Some('n') => '\n',
                Some('t') => '\t',
                Some('u') => {
                    let hex: String = chars.by_ref().take(4).collect();
                    u32::from_str_radix(&hex, 16)
                        .ok()
                        .filter(|_| hex.len() == 4)
                        .and_then(char::from_u32)
                        .ok_or_else(|| format!("`\\u{hex}` is not a character"))?
                }
                other => return Err(format!("unknown escape `\\{}`", other.unwrap_or(' '))),
            },
            c => c,
        };
        if c == '\0' {
            return Err(
                "U+0000 cannot be compiled into a resource (rcomp's <0x0000> is broken)".into(),
            );
        }
        out.push(c);
    }
    Ok(out)
}
