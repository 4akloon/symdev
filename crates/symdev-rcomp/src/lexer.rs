//! Tokens of preprocessed resource source (`.rpp`): what `rcomp` reads after `cpp`.

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

/// How source bytes become characters (spec §5.6): CP1252 unless `CHARACTER_SET` says
/// otherwise; `ISOLATIN1`, `ASCII` and `CP850` all behave as ISO 8859-1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RssCharset {
    #[default]
    Cp1252,
    Latin1,
    Utf8,
}

impl RssCharset {
    /// CP1252 differs from ISO 8859-1 only in 0x80..=0x9F.
    const CP1252_HIGH: [u16; 32] = [
        0x20ac, 0x0081, 0x201a, 0x0192, 0x201e, 0x2026, 0x2020, 0x2021, 0x02c6, 0x2030, 0x0160,
        0x2039, 0x0152, 0x008d, 0x017d, 0x008f, 0x0090, 0x2018, 0x2019, 0x201c, 0x201d, 0x2022,
        0x2013, 0x2014, 0x02dc, 0x2122, 0x0161, 0x203a, 0x0153, 0x009d, 0x017e, 0x0178,
    ];

    /// The `CHARACTER_SET` statement of a preprocessed source, if it has one.
    pub fn of(src: &[u8]) -> Result<Self> {
        let text = String::from_utf8_lossy(src);
        let Some(at) = text.find("CHARACTER_SET") else {
            return Ok(Self::default());
        };
        let name = text[at + "CHARACTER_SET".len()..]
            .split_whitespace()
            .next()
            .unwrap_or("")
            .to_ascii_uppercase();
        match name.as_str() {
            "CP1252" => Ok(Self::Cp1252),
            "ISOLATIN1" | "ASCII" | "CP850" => Ok(Self::Latin1),
            "UTF8" => Ok(Self::Utf8),
            other => Err(Error::Other(format!(
                "TODO: CHARACTER_SET {other} (rcomp rejects UNICODE and we did not observe SHIFTJIS)"
            ))),
        }
    }

    /// Characters of literal source bytes.
    fn decode(self, bytes: &[u8]) -> Vec<u32> {
        match self {
            Self::Utf8 => String::from_utf8_lossy(bytes)
                .chars()
                .map(u32::from)
                .collect(),
            Self::Latin1 => bytes.iter().map(|&b| u32::from(b)).collect(),
            Self::Cp1252 => bytes
                .iter()
                .map(|&b| match b {
                    0x80..=0x9f => u32::from(Self::CP1252_HIGH[usize::from(b) - 0x80]),
                    _ => u32::from(b),
                })
                .collect(),
        }
    }
}

/// Splits `.rpp` text into tokens. `# <line> "<file>"` markers set the position;
/// comments are skipped. Source bytes are Latin-1 (no `CHARACTER_SET` in the SDK
/// examples, experiment 56).
pub struct RssLexer<'a> {
    src: &'a [u8],
    at: usize,
    file: String,
    line: u32,
    charset: RssCharset,
}

impl<'a> RssLexer<'a> {
    pub fn new(src: &'a [u8], file: &str) -> Result<Self> {
        Ok(Self {
            src,
            at: 0,
            file: file.to_string(),
            line: 1,
            charset: RssCharset::of(src)?,
        })
    }

    pub fn tokens(mut self) -> Result<Vec<RssSpanned>> {
        let mut out = Vec::new();
        let mut line_start = true;
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
mod tests {
    use super::*;

    fn kinds(src: &str) -> Vec<RssToken> {
        RssLexer::new(src.as_bytes(), "t.rss")
            .unwrap()
            .tokens()
            .unwrap()
            .into_iter()
            .map(|t| t.token)
            .collect()
    }

    #[test]
    fn lexes_resource_statement_with_comments_and_markers() {
        let toks = kinds(
            "# 1 \"a.rss\"\nNAME TEST // id\n/* c */ RESOURCE S r { b = 0x10; t = \"H\\\"i\"; d = -0.25; c = 'A'; }\n",
        );
        assert_eq!(toks[0], RssToken::Ident("NAME".into()));
        assert!(toks.contains(&RssToken::Int(16)));
        assert!(toks.contains(&RssToken::Str(vec![0x48, 0x22, 0x69])));
        assert!(toks.contains(&RssToken::Real(0.25)));
        assert!(toks.contains(&RssToken::Char(0x41)));
    }

    #[test]
    fn line_markers_set_file_and_line() {
        let toks = RssLexer::new(b"# 7 \"Z:\\\\x\\\\y.rh\" 1\nfoo\n", "t")
            .unwrap()
            .tokens()
            .unwrap();
        assert_eq!(toks[0].file, "Z:\\x\\y.rh");
        assert_eq!(toks[0].line, 7);
    }
}
