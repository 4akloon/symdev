//! A cursor over the source text of a token stream, for reading a function header.
//!
//! The attribute cannot inspect its input with the `proc_macro` API and still be
//! testable: `TokenStream` panics the moment it is touched outside a real macro
//! expansion, so a `#[test]` can never build one. The logic therefore reads
//! `item.to_string()`, and only the *generated* wrapper is turned back into tokens —
//! the user's function is re-emitted as the token stream it arrived as, so its spans
//! and its body are untouched.
//!
//! What this has to survive is a token stream's own rendering: attributes (`#[doc =
//! "…"]`), string and raw-string literals, character literals and lifetimes. It never
//! sees a function body, because [`crate::signature::Signature`] stops at the
//! parameter list.

/// A position in the rendered token stream, with the skips a header scan needs.
pub struct Cursor<'a> {
    source: &'a str,
    at: usize,
}

impl<'a> Cursor<'a> {
    pub fn new(source: &'a str) -> Self {
        Self { source, at: 0 }
    }

    pub fn peek(&self) -> Option<char> {
        self.source[self.at..].chars().next()
    }

    /// The rest of the input, for a message that has to show what was found.
    pub fn rest(&self) -> &'a str {
        &self.source[self.at..]
    }

    fn bump(&mut self) {
        if let Some(c) = self.peek() {
            self.at += c.len_utf8();
        }
    }

    /// Whitespace and comments. Comments are here because a token stream renders as
    /// the source it came from when nothing has rewritten it, so `/// a doc comment`
    /// and `// a note` arrive exactly as the user typed them.
    pub fn skip_space(&mut self) {
        loop {
            while self.peek().is_some_and(char::is_whitespace) {
                self.bump();
            }
            if !self.skip_comment() {
                return;
            }
        }
    }

    /// `// …` to the end of the line, or a `/* … */` that may nest.
    fn skip_comment(&mut self) -> bool {
        let rest = self.rest();
        if rest.starts_with("//") {
            self.at += rest.find('\n').map_or(rest.len(), |end| end + 1);
            return true;
        }
        if !rest.starts_with("/*") {
            return false;
        }
        self.at += 2;
        let mut depth = 1usize;
        while depth > 0 {
            let rest = self.rest();
            if rest.starts_with("*/") {
                depth -= 1;
                self.at += 2;
            } else if rest.starts_with("/*") {
                depth += 1;
                self.at += 2;
            } else if rest.is_empty() {
                return true;
            } else {
                self.bump();
            }
        }
        true
    }

    /// The identifier at the cursor, empty if there is none.
    pub fn ident(&mut self) -> &'a str {
        let start = self.at;
        if !self.peek().is_some_and(is_ident_start) {
            return "";
        }
        while self.peek().is_some_and(is_ident_continue) {
            self.bump();
        }
        &self.source[start..self.at]
    }

    /// Steps over a literal, a lifetime or one ordinary character, so that nothing
    /// inside a string can be mistaken for a delimiter.
    pub fn skip_atom(&mut self) {
        match self.peek() {
            Some('"') => self.skip_string(),
            Some('r') if self.raw_string_ahead() => self.skip_raw_string(),
            Some('\'') => self.skip_quote(),
            Some('/') if self.skip_comment() => {}
            _ => self.bump(),
        }
    }

    fn skip_string(&mut self) {
        self.bump();
        while let Some(c) = self.peek() {
            self.bump();
            match c {
                '\\' => self.bump(),
                '"' => return,
                _ => {}
            }
        }
    }

    fn raw_string_ahead(&self) -> bool {
        match self.source[self.at..].strip_prefix('r') {
            Some(after) => after.trim_start_matches('#').starts_with('"'),
            None => false,
        }
    }

    /// `r"…"` or `r#"…"#`: the closing quote is followed by as many `#` as opened it.
    fn skip_raw_string(&mut self) {
        self.bump();
        let mut hashes = 0;
        while self.peek() == Some('#') {
            hashes += 1;
            self.bump();
        }
        if self.peek() != Some('"') {
            return;
        }
        self.bump();
        let close = format!("\"{}", "#".repeat(hashes));
        match self.source[self.at..].find(&close) {
            Some(end) => self.at += end + close.len(),
            None => self.at = self.source.len(),
        }
    }

    /// A `'`: either a character literal or a lifetime. `'a'` and `'\n'` are literals;
    /// `'a` in `&'a str` is a lifetime and only the tick is consumed.
    fn skip_quote(&mut self) {
        let rest = &self.source[self.at + 1..];
        let mut chars = rest.chars();
        let literal = matches!(
            (chars.next(), chars.next()),
            (Some('\\'), _) | (Some(_), Some('\''))
        );
        self.bump();
        if !literal {
            return;
        }
        while let Some(c) = self.peek() {
            self.bump();
            match c {
                '\\' => self.bump(),
                '\'' => return,
                _ => {}
            }
        }
    }

    /// The text inside the group that starts at the cursor, with the cursor left after
    /// its closing delimiter. `None` when the group is never closed.
    pub fn group(&mut self, open: char, close: char) -> Option<&'a str> {
        if self.peek() != Some(open) {
            return None;
        }
        self.bump();
        let start = self.at;
        let mut depth = 1usize;
        while let Some(c) = self.peek() {
            if c == open {
                depth += 1;
            } else if c == close {
                depth -= 1;
                if depth == 0 {
                    let inner = &self.source[start..self.at];
                    self.bump();
                    return Some(inner);
                }
            }
            self.skip_atom();
        }
        None
    }

    /// One outer attribute, `#[…]` or `#![…]`.
    pub fn skip_attribute(&mut self) -> Option<()> {
        self.bump();
        if self.peek() == Some('!') {
            self.bump();
        }
        self.group('[', ']').map(|_| ())
    }
}

fn is_ident_start(c: char) -> bool {
    c == '_' || c.is_alphabetic()
}

fn is_ident_continue(c: char) -> bool {
    c == '_' || c.is_alphanumeric()
}
