//! `ProjectLine`: one tokenised line of a preprocessed `bld.inf` or `.mmp`.

/// One non-blank line of preprocessed project-file text, with the source file and line
/// number the `# <n> "<file>"` markers gave it (mmp-frontend-spec.md §1.7, §2.3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectLine {
    pub file: String,
    pub number: usize,
    pub tokens: Vec<String>,
}

impl ProjectLine {
    /// The records of one preprocessed file: markers consumed, blank lines dropped.
    pub fn records(text: &str) -> Vec<Self> {
        let mut out = Vec::new();
        let mut file = String::new();
        let mut number = 1usize;
        for raw in text.lines() {
            if let Some((at, name)) = Self::marker(raw) {
                number = at;
                file = name;
                continue;
            }
            let tokens = Self::tokens(raw);
            if !tokens.is_empty() {
                out.push(Self {
                    file: file.clone(),
                    number,
                    tokens,
                });
            }
            number += 1;
        }
        out
    }

    /// The directive name, upper-cased as the SDK matches it (§2.4).
    pub fn directive(&self) -> String {
        self.tokens[0].to_ascii_uppercase()
    }

    /// The arguments, in source case.
    pub fn args(&self) -> &[String] {
        &self.tokens[1..]
    }

    /// `<file>:<line>`, for error messages.
    pub fn at(&self) -> String {
        format!("{}:{}", self.file, self.number)
    }

    /// `# <number> "<file>"`, optionally followed by flags.
    fn marker(line: &str) -> Option<(usize, String)> {
        let rest = line.strip_prefix('#')?.trim_start();
        let (digits, rest) = rest.split_at(rest.find(|c: char| !c.is_ascii_digit())?);
        let number: usize = digits.parse().ok()?;
        let name = rest.trim_start().strip_prefix('"')?;
        Some((number, name.split('"').next()?.to_string()))
    }

    /// A double-quoted run (without its quotes) or a run of non-separator characters
    /// (§2.3). A quote inside a word ends the word; `""` is dropped.
    fn tokens(line: &str) -> Vec<String> {
        let chars: Vec<char> = line.chars().collect();
        let separator = |c: char| matches!(c, ' ' | '\t' | '\r' | '\n' | '\x0c');
        let mut out = Vec::new();
        let mut i = 0;
        while i < chars.len() {
            if separator(chars[i]) {
                i += 1;
                continue;
            }
            if chars[i] == '"' {
                // A space is legal inside a quoted run; a tab or a newline is not.
                let start = i + 1;
                let mut end = start;
                while end < chars.len() && !matches!(chars[end], '"' | '\t' | '\r' | '\n' | '\x0c')
                {
                    end += 1;
                }
                if end > start && chars.get(end) == Some(&'"') {
                    out.push(chars[start..end].iter().collect());
                    i = end + 1;
                } else {
                    i += 1;
                }
                continue;
            }
            let start = i;
            while i < chars.len() && !separator(chars[i]) && chars[i] != '"' {
                i += 1;
            }
            out.push(chars[start..i].iter().collect());
        }
        out
    }
}
