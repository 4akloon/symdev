//! Preprocessing tokens: source text after line splicing and comment removal.

/// A preprocessing token. Whitespace is kept (one `Space` per run) so the output reads
/// like the input and tokens never glue together.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CppToken {
    Ident(String),
    /// pp-number, string or character literal, or punctuator: copied as written.
    Other(String),
    Space,
    Newline,
}

impl CppToken {
    pub fn text(&self) -> &str {
        match self {
            CppToken::Ident(s) | CppToken::Other(s) => s,
            CppToken::Space => " ",
            CppToken::Newline => "\n",
        }
    }

    pub fn is_space(&self) -> bool {
        matches!(self, CppToken::Space)
    }
}

/// Source lines of one file with backslash-newlines spliced and comments replaced by a
/// space (newlines inside block comments kept, so line numbers stay right).
pub struct CppSource;

impl CppSource {
    pub fn clean(src: &[u8]) -> String {
        // Latin-1: every byte is one char (`rcomp` reads 8-bit source).
        let text: String = src.iter().map(|&b| char::from(b)).collect();
        let text = text.replace("\r\n", "\n").replace("\\\n", "");
        let mut out = String::with_capacity(text.len());
        let mut chars = text.chars().peekable();
        while let Some(c) = chars.next() {
            match c {
                '"' | '\'' => {
                    out.push(c);
                    while let Some(d) = chars.next() {
                        out.push(d);
                        if d == '\\' {
                            if let Some(e) = chars.next() {
                                out.push(e);
                            }
                        } else if d == c || d == '\n' {
                            break;
                        }
                    }
                }
                '/' if chars.peek() == Some(&'/') => {
                    while chars.peek().is_some_and(|&d| d != '\n') {
                        chars.next();
                    }
                    out.push(' ');
                }
                '/' if chars.peek() == Some(&'*') => {
                    chars.next();
                    let mut prev = ' ';
                    for d in chars.by_ref() {
                        if d == '\n' {
                            out.push('\n');
                        }
                        if prev == '*' && d == '/' {
                            break;
                        }
                        prev = d;
                    }
                    out.push(' ');
                }
                _ => out.push(c),
            }
        }
        out
    }

    /// Tokens of one line (no `Newline`).
    pub fn tokens(line: &str) -> Vec<CppToken> {
        let chars: Vec<char> = line.chars().collect();
        let mut out = Vec::new();
        let mut i = 0;
        while i < chars.len() {
            let c = chars[i];
            let start = i;
            if c == ' ' || c == '\t' || c == '\r' || c == '\x0c' || c == '\x0b' {
                while i < chars.len() && matches!(chars[i], ' ' | '\t' | '\r' | '\x0c' | '\x0b') {
                    i += 1;
                }
                if !matches!(out.last(), Some(CppToken::Space)) {
                    out.push(CppToken::Space);
                }
                continue;
            }
            if c.is_ascii_alphabetic() || c == '_' {
                while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
                    i += 1;
                }
                out.push(CppToken::Ident(chars[start..i].iter().collect()));
                continue;
            }
            if c.is_ascii_digit()
                || (c == '.' && chars.get(i + 1).is_some_and(char::is_ascii_digit))
            {
                while i < chars.len()
                    && (chars[i].is_ascii_alphanumeric()
                        || chars[i] == '.'
                        || chars[i] == '_'
                        || (matches!(chars[i], '+' | '-')
                            && matches!(chars[i - 1], 'e' | 'E')
                            && !chars[start..i].iter().any(|c| matches!(c, 'x' | 'X'))))
                {
                    i += 1;
                }
                out.push(CppToken::Other(chars[start..i].iter().collect()));
                continue;
            }
            if c == '"' || c == '\'' {
                i += 1;
                while i < chars.len() && chars[i] != c {
                    if chars[i] == '\\' {
                        i += 1;
                    }
                    i += 1;
                }
                i = (i + 1).min(chars.len());
                out.push(CppToken::Other(chars[start..i].iter().collect()));
                continue;
            }
            let two: String = chars[i..(i + 2).min(chars.len())].iter().collect();
            let len = match two.as_str() {
                "##" | "&&" | "||" | "==" | "!=" | "<=" | ">=" | "<<" | ">>" => 2,
                _ => 1,
            };
            i += len;
            out.push(CppToken::Other(chars[start..i].iter().collect()));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_drops_comments_keeps_strings_and_lines() {
        let s = CppSource::clean(b"a // x\r\nb /* y\nz */ c \"//no\"\nd\\\ne");
        assert_eq!(s, "a  \nb \n  c \"//no\"\nde");
    }

    #[test]
    fn tokens_split_idents_numbers_strings_and_punct() {
        let t = CppSource::tokens("#define F(a,b) a##b 0x10 \"s\"");
        let texts: Vec<&str> = t.iter().map(CppToken::text).collect();
        assert_eq!(
            texts,
            [
                "#", "define", " ", "F", "(", "a", ",", "b", ")", " ", "a", "##", "b", " ", "0x10",
                " ", "\"s\""
            ]
        );
    }
}
