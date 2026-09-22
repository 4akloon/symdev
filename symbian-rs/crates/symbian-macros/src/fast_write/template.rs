//! A format string's value, split into text and holes.
//!
//! Only the holes a fast `write!` can take apart are accepted: `{}`, `{0}`, `{name}`,
//! each with no format spec (a bare `{:}` counts as none). A width, a precision, a
//! fill, `#`, `?`, `x` — any of them — is `None`, and the whole invocation is left to
//! `core::write!`, whose diagnostics then point at the user's string as they always
//! did. So is anything malformed: rustc, not this module, is the one to say so.

/// Which argument a hole names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Hole {
    /// `{}`: the next positional argument.
    Next,
    /// `{0}`.
    Index(usize),
    /// `{name}`: a named argument, or a variable captured from the caller's scope.
    Name(String),
}

/// One run of a format string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Segment {
    Text(String),
    Hole(Hole),
}

/// The segments of `value`, or `None` if any hole has a spec or anything is malformed.
pub fn segments(value: &str) -> Option<Vec<Segment>> {
    let mut out = Vec::new();
    let mut text = String::new();
    let mut chars = value.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '{' if chars.next_if_eq(&'{').is_some() => text.push('{'),
            '}' if chars.next_if_eq(&'}').is_some() => text.push('}'),
            '}' => return None,
            '{' => {
                let mut inner = String::new();
                loop {
                    match chars.next()? {
                        '}' => break,
                        '{' => return None,
                        other => inner.push(other),
                    }
                }
                if !text.is_empty() {
                    out.push(Segment::Text(std::mem::take(&mut text)));
                }
                out.push(Segment::Hole(hole(&inner)?));
            }
            other => text.push(other),
        }
    }
    if !text.is_empty() {
        out.push(Segment::Text(text));
    }
    Some(out)
}

/// The inside of one `{…}`.
fn hole(inner: &str) -> Option<Hole> {
    let argument = match inner.split_once(':') {
        Some((argument, "")) => argument,
        Some(_) => return None,
        None => inner,
    };
    if argument.is_empty() {
        return Some(Hole::Next);
    }
    if argument.bytes().all(|b| b.is_ascii_digit()) {
        if argument.len() > 1 && argument.starts_with('0') {
            return None;
        }
        return argument.parse().ok().map(Hole::Index);
    }
    is_capturable(argument).then(|| Hole::Name(argument.to_owned()))
}

/// An ASCII identifier that is not a keyword and not `_`. Anything else — a raw
/// identifier, a non-ASCII one, `self` — goes to `core::write!` untouched.
fn is_capturable(name: &str) -> bool {
    let mut bytes = name.bytes();
    let starts = bytes
        .next()
        .is_some_and(|b| b.is_ascii_alphabetic() || b == b'_');
    starts
        && bytes.all(|b| b.is_ascii_alphanumeric() || b == b'_')
        && name != "_"
        && !KEYWORDS.contains(&name)
}

const KEYWORDS: &[&str] = &[
    "Self", "abstract", "as", "async", "await", "become", "box", "break", "const", "continue",
    "crate", "do", "dyn", "else", "enum", "extern", "false", "final", "fn", "for", "gen", "if",
    "impl", "in", "let", "loop", "macro", "match", "mod", "move", "mut", "override", "priv",
    "pub", "ref", "return", "self", "static", "struct", "super", "trait", "true", "try",
    "type", "typeof", "union", "unsafe", "unsized", "use", "virtual", "where", "while",
    "yield",
];
