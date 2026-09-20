//! A JSON reader, only as much of one as the result protocol needs.
//!
//! The file this parses is written by `symbian_std::test_report` on the emulated
//! device, so both halves are ours; but it comes back off a drive image after a
//! process that may have died half-way through writing it, so it is parsed strictly
//! and a malformed file is an error rather than a silent pass.
//!
//! No dependency is added for this: the workspace has no JSON crate, the grammar below
//! is the whole of RFC 8259's value syntax that the protocol uses, and a parser is
//! cheaper than a supply chain.
use std::collections::BTreeMap;

use symdev_core::{Error, Result};

/// A parsed JSON value.
#[derive(Debug, Clone, PartialEq)]
pub enum Json {
    Null,
    Bool(bool),
    /// Every number in the protocol is a small non-negative count, so one `i64` is
    /// enough and a fractional number is refused rather than rounded.
    Number(i64),
    String(String),
    Array(Vec<Json>),
    Object(BTreeMap<String, Json>),
}

impl Json {
    /// Parses one JSON document. Trailing text other than whitespace is an error.
    pub fn parse(text: &str) -> Result<Self> {
        let mut p = Parser {
            rest: text.as_bytes(),
            at: 0,
        };
        p.skip_space();
        let value = p.value()?;
        p.skip_space();
        if p.at != p.rest.len() {
            return Err(p.fail("trailing text after the JSON document"));
        }
        Ok(value)
    }

    /// The member of an object, if this is an object and it has one.
    pub fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Self::Object(map) => map.get(key),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Self::Number(n) => Some(*n),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&[Json]> {
        match self {
            Self::Array(items) => Some(items),
            _ => None,
        }
    }
}

struct Parser<'a> {
    rest: &'a [u8],
    at: usize,
}

impl Parser<'_> {
    fn fail(&self, what: &str) -> Error {
        Error::Other(format!("test result file: {what} at byte {}", self.at))
    }

    fn peek(&self) -> Option<u8> {
        self.rest.get(self.at).copied()
    }

    fn skip_space(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.at += 1;
        }
    }

    fn eat(&mut self, byte: u8) -> Result<()> {
        if self.peek() == Some(byte) {
            self.at += 1;
            return Ok(());
        }
        Err(self.fail(&format!("expected `{}`", byte as char)))
    }

    fn literal(&mut self, word: &str, value: Json) -> Result<Json> {
        if self.rest[self.at..].starts_with(word.as_bytes()) {
            self.at += word.len();
            return Ok(value);
        }
        Err(self.fail("unrecognised literal"))
    }

    fn value(&mut self) -> Result<Json> {
        match self.peek() {
            Some(b'{') => self.object(),
            Some(b'[') => self.array(),
            Some(b'"') => self.string().map(Json::String),
            Some(b't') => self.literal("true", Json::Bool(true)),
            Some(b'f') => self.literal("false", Json::Bool(false)),
            Some(b'n') => self.literal("null", Json::Null),
            Some(c) if c == b'-' || c.is_ascii_digit() => self.number(),
            _ => Err(self.fail("expected a JSON value")),
        }
    }

    fn object(&mut self) -> Result<Json> {
        self.eat(b'{')?;
        let mut map = BTreeMap::new();
        self.skip_space();
        if self.peek() == Some(b'}') {
            self.at += 1;
            return Ok(Json::Object(map));
        }
        loop {
            self.skip_space();
            let key = self.string()?;
            self.skip_space();
            self.eat(b':')?;
            self.skip_space();
            map.insert(key, self.value()?);
            self.skip_space();
            match self.peek() {
                Some(b',') => self.at += 1,
                Some(b'}') => {
                    self.at += 1;
                    return Ok(Json::Object(map));
                }
                _ => return Err(self.fail("expected `,` or `}`")),
            }
        }
    }

    fn array(&mut self) -> Result<Json> {
        self.eat(b'[')?;
        let mut items = Vec::new();
        self.skip_space();
        if self.peek() == Some(b']') {
            self.at += 1;
            return Ok(Json::Array(items));
        }
        loop {
            self.skip_space();
            items.push(self.value()?);
            self.skip_space();
            match self.peek() {
                Some(b',') => self.at += 1,
                Some(b']') => {
                    self.at += 1;
                    return Ok(Json::Array(items));
                }
                _ => return Err(self.fail("expected `,` or `]`")),
            }
        }
    }

    fn string(&mut self) -> Result<String> {
        self.eat(b'"')?;
        let start = self.at;
        let mut out = String::new();
        while let Some(c) = self.peek() {
            match c {
                b'"' => {
                    self.at += 1;
                    return Ok(out);
                }
                b'\\' => {
                    self.at += 1;
                    out.push(self.escape()?);
                }
                _ => {
                    // Multi-byte UTF-8 is copied through whole: find the next byte that
                    // starts a character or ends the string.
                    let from = self.at;
                    self.at += 1;
                    while matches!(self.peek(), Some(c) if c & 0xc0 == 0x80) {
                        self.at += 1;
                    }
                    match std::str::from_utf8(&self.rest[from..self.at]) {
                        Ok(s) => out.push_str(s),
                        Err(_) => return Err(self.fail("the string is not UTF-8")),
                    }
                }
            }
        }
        self.at = start;
        Err(self.fail("unterminated string"))
    }

    fn escape(&mut self) -> Result<char> {
        let c = self.peek().ok_or_else(|| self.fail("escape at end"))?;
        self.at += 1;
        Ok(match c {
            b'"' => '"',
            b'\\' => '\\',
            b'/' => '/',
            b'b' => '\u{8}',
            b'f' => '\u{c}',
            b'n' => '\n',
            b'r' => '\r',
            b't' => '\t',
            b'u' => return self.escape_u(),
            _ => return Err(self.fail("unrecognised escape")),
        })
    }

    /// `\uXXXX`. A surrogate arrives only from a writer that produced one; the protocol
    /// never does, so one is an error rather than a guess at the pair.
    fn escape_u(&mut self) -> Result<char> {
        let end = self.at + 4;
        let digits = self
            .rest
            .get(self.at..end)
            .and_then(|d| std::str::from_utf8(d).ok())
            .and_then(|d| u32::from_str_radix(d, 16).ok())
            .ok_or_else(|| self.fail("bad \\u escape"))?;
        self.at = end;
        char::from_u32(digits).ok_or_else(|| self.fail("\\u escape is not a character"))
    }

    fn number(&mut self) -> Result<Json> {
        let start = self.at;
        if self.peek() == Some(b'-') {
            self.at += 1;
        }
        while matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
            self.at += 1;
        }
        if matches!(self.peek(), Some(b'.' | b'e' | b'E')) {
            return Err(self.fail("the protocol has no fractional numbers"));
        }
        std::str::from_utf8(&self.rest[start..self.at])
            .ok()
            .and_then(|s| s.parse::<i64>().ok())
            .map(Json::Number)
            .ok_or_else(|| self.fail("not a number"))
    }
}

#[cfg(test)]
mod tests;
