//! Tokens of preprocessed resource source (`.rpp`): what `rcomp` reads after `cpp`.

mod charset;

pub use charset::RssCharset;

use symdev_core::{Error, Result};

#[derive(Debug, Clone, PartialEq)]
pub enum RssToken {
    Ident(String),
    Int(i64),
    Real(f64),
    /// String literal, escapes resolved, as Unicode scalar values.
    Str(Vec<u32>),
    /// `'c'` character constant.
    Char(u32),
    Punct(char),
}

/// A token and the source line it came from (for error messages).
#[derive(Debug, Clone, PartialEq)]
pub struct RssSpanned {
    pub token: RssToken,
    pub file: String,
    pub line: u32,
}

/// Splits `.rpp` text into tokens. `# <line> "<file>"` markers set the position;
/// comments are skipped. Source bytes are CP1252 until a `CHARACTER_SET` statement
/// says otherwise (spec §5.6).
pub struct RssLexer<'a> {
    src: &'a [u8],
    at: usize,
    file: String,
    line: u32,
    charset: RssCharset,
}

impl<'a> RssLexer<'a> {
    pub fn new(src: &'a [u8], file: &str) -> Self {
        Self {
            src,
            at: 0,
            file: file.to_string(),
            line: 1,
            charset: RssCharset::default(),
        }
    }

    pub fn tokens(mut self) -> Result<Vec<RssSpanned>> {
        let mut out = Vec::new();
        let mut line_start = true;
        let mut expect_charset = false;
        while self.at < self.src.len() {
            let c = self.src[self.at];
            if c == b'\n' {
                self.line += 1;
                self.at += 1;
                line_start = true;
                continue;
            }
            if c == b' ' || c == b'\t' || c == b'\r' || c == 0x0c {
                self.at += 1;
                continue;
            }
            if line_start && c == b'#' {
                self.directive()?;
                continue;
            }
            line_start = false;
            if c == b'/' && self.peek(1) == Some(b'/') {
                while self.at < self.src.len() && self.src[self.at] != b'\n' {
                    self.at += 1;
                }
                continue;
            }
            if c == b'/' && self.peek(1) == Some(b'*') {
                self.at += 2;
                loop {
                    match self.src.get(self.at) {
                        None => return Err(self.error("unterminated comment")),
                        Some(b'*') if self.peek(1) == Some(b'/') => {
                            self.at += 2;
                            break;
                        }
                        Some(b'\n') => {
                            self.line += 1;
                            self.at += 1;
                        }
                        Some(_) => self.at += 1,
                    }
                }
                continue;
            }
            let line = self.line;
            let token = if c.is_ascii_alphabetic() || c == b'_' {
                let start = self.at;
                while self
                    .src
                    .get(self.at)
                    .is_some_and(|b| b.is_ascii_alphanumeric() || *b == b'_')
                {
                    self.at += 1;
                }
                RssToken::Ident(String::from_utf8_lossy(&self.src[start..self.at]).into_owned())
            } else if c.is_ascii_digit()
                || (c == b'.' && self.peek(1).is_some_and(|b| b.is_ascii_digit()))
            {
                self.number()?
            } else if c == b'"' {
                RssToken::Str(self.quoted(b'"')?)
            } else if c == b'\'' {
                let chars = self.quoted(b'\'')?;
                match chars[..] {
                    [one] => RssToken::Char(one),
                    _ => return Err(self.error("character constant must be one character")),
                }
            } else {
                self.at += 1;
                RssToken::Punct(char::from(c))
            };
            // `CHARACTER_SET <name>` changes how the literals after it are decoded.
            if let RssToken::Ident(name) = &token {
                if std::mem::take(&mut expect_charset) {
                    self.charset =
                        RssCharset::from_name(name).map_err(|e| self.error(&e.to_string()))?;
                } else {
                    expect_charset = name == "CHARACTER_SET";
                }
            }
            out.push(RssSpanned {
                token,
                file: self.file.clone(),
                line,
            });
        }
        Ok(out)
    }

    fn peek(&self, n: usize) -> Option<u8> {
        self.src.get(self.at + n).copied()
    }

    fn error(&self, what: &str) -> Error {
        Error::Other(format!("{}({}): {what}", self.file, self.line))
    }

    /// `# 12 "file" flags` (cpp line marker) or `#line 12 "file"`; other directives are
    /// errors (the source must already be preprocessed).
    fn directive(&mut self) -> Result<()> {
        let end = self.src[self.at..]
            .iter()
            .position(|&b| b == b'\n')
            .map_or(self.src.len(), |p| self.at + p);
        let text = String::from_utf8_lossy(&self.src[self.at + 1..end]).into_owned();
        self.at = end;
        let text = text.trim();
        let text = text.strip_prefix("line").unwrap_or(text).trim_start();
        let mut parts = text.splitn(2, char::is_whitespace);
        let line: u32 = parts
            .next()
            .and_then(|n| n.parse().ok())
            .ok_or_else(|| self.error(&format!("unexpected directive #{text}")))?;
        if let Some(rest) = parts.next()
            && let Some(q) = rest.trim_start().strip_prefix('"')
            && let Some(close) = q.rfind('"')
        {
            self.file = q[..close].replace("\\\\", "\\");
        }
        // The newline ending the marker is counted next, so this line is `line - 1`.
        self.line = line.saturating_sub(1);
        Ok(())
    }

    fn number(&mut self) -> Result<RssToken> {
        let start = self.at;
        if self.src[self.at] == b'0' && matches!(self.peek(1), Some(b'x' | b'X')) {
            self.at += 2;
            let digits = self.at;
            while self.src.get(self.at).is_some_and(u8::is_ascii_hexdigit) {
                self.at += 1;
            }
            let hex = std::str::from_utf8(&self.src[digits..self.at]).unwrap_or("");
            let value = u64::from_str_radix(hex, 16)
                .map_err(|_| self.error(&format!("bad hex number 0x{hex}")))?;
            self.skip_suffix();
            return Ok(RssToken::Int(value as i64));
        }
        let mut real = false;
        while let Some(b) = self.src.get(self.at) {
            match b {
                b'0'..=b'9' => {}
                b'.' => real = true,
                b'e' | b'E' if real => {
                    if matches!(self.peek(1), Some(b'-' | b'+')) {
                        self.at += 1;
                    }
                }
                _ => break,
            }
            self.at += 1;
        }
        let text = std::str::from_utf8(&self.src[start..self.at]).unwrap_or("");
        if real {
            let value = text
                .parse::<f64>()
                .map_err(|_| self.error(&format!("bad number {text}")))?;
            return Ok(RssToken::Real(value));
        }
        let value = text
            .parse::<i64>()
            .map_err(|_| self.error(&format!("bad number {text}")))?;
        self.skip_suffix();
        Ok(RssToken::Int(value))
    }

    /// C integer suffixes (`0x10000000u`, `1L`).
    fn skip_suffix(&mut self) {
        while matches!(self.src.get(self.at), Some(b'u' | b'U' | b'l' | b'L')) {
            self.at += 1;
        }
    }

    fn quoted(&mut self, close: u8) -> Result<Vec<u32>> {
        self.at += 1;
        let mut out = Vec::new();
        loop {
            let Some(&c) = self.src.get(self.at) else {
                return Err(self.error("unterminated literal"));
            };
            self.at += 1;
            match c {
                b'\n' => return Err(self.error("newline in literal")),
                _ if c == close => return Ok(out),
                b'\\' => {
                    let Some(&e) = self.src.get(self.at) else {
                        return Err(self.error("unterminated literal"));
                    };
                    self.at += 1;
                    out.push(match e {
                        b'n' => 0x0a,
                        b't' => 0x09,
                        b'r' => 0x0d,
                        b'\\' => 0x5c,
                        b'"' => 0x22,
                        b'\'' => 0x27,
                        // Experiment 56: richtexteditor's "\f" is U+000C in the `.rsc`.
                        b'f' => 0x0c,
                        other => {
                            return Err(self.error(&format!(
                                "TODO: escape \\{} (not observed)",
                                char::from(other)
                            )));
                        }
                    });
                }
                _ => {
                    let start = self.at - 1;
                    while self
                        .src
                        .get(self.at)
                        .is_some_and(|&b| b != close && b != b'\\' && b != b'\n')
                    {
                        self.at += 1;
                    }
                    out.extend(self.charset.decode(&self.src[start..self.at]));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests;
