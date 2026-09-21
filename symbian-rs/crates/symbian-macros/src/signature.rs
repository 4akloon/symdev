//! `Signature`: the header of the function the attribute was applied to.
//!
//! Only as much of it as the entry point cares about — the qualifiers, the name, the
//! parameter list and, for a GUI application, the return type.
//!
//! For a console entry the return type is deliberately *not* looked at: whether it can
//! become a `TInt` is `IntoExitCode`'s question, and rustc asks it better than a macro
//! could. A GUI entry is the exception, because the type it returns is the application
//! type the vtable has to be built for, and only the signature says which it is.

use crate::cursor::Cursor;

/// The qualifiers a `fn` may carry before the attribute has to complain. `async` is
/// not among them and neither is `unsafe`: both are rejected by name, with a reason.
const QUALIFIERS: &[&str] = &["pub", "const", "async", "unsafe", "extern", "default"];

/// What was written between the attribute and the function body.
pub struct Signature<'a> {
    /// The words before `fn`, in the order they appeared.
    pub qualifiers: Vec<&'a str>,
    /// The function's own name.
    pub name: &'a str,
    /// The text between the parentheses, trimmed; empty when it takes none.
    pub parameters: &'a str,
    /// What follows `->`, trimmed, up to the `where` clause or the body; `None` when
    /// the function returns `()` by writing nothing.
    pub returns: Option<&'a str>,
}

impl<'a> Signature<'a> {
    /// Reads the header out of the rendered item, or says what was found instead.
    pub fn parse(item: &'a str, attribute: &str) -> Result<Self, String> {
        let mut cursor = Cursor::new(item);
        let mut qualifiers = Vec::new();
        loop {
            cursor.skip_space();
            match cursor.peek() {
                Some('#') => {
                    cursor
                        .skip_attribute()
                        .ok_or_else(|| format!("`{attribute}`: unclosed attribute"))?;
                }
                Some(_) => {
                    let word = cursor.ident();
                    if word == "fn" {
                        break;
                    }
                    if word.is_empty() || !QUALIFIERS.contains(&word) {
                        return Err(not_a_function(attribute, word, cursor.rest()));
                    }
                    qualifiers.push(word);
                    skip_qualifier_tail(&mut cursor, word);
                }
                None => return Err(not_a_function(attribute, "", "")),
            }
        }
        cursor.skip_space();
        let name = cursor.ident();
        if name.is_empty() {
            return Err(format!("`{attribute}`: this function has no name"));
        }
        cursor.skip_space();
        if cursor.peek() == Some('<') {
            return Err(format!(
                "`{attribute}`: `fn {name}` cannot be generic — the process entry point \
                 `E32Main()` is one concrete function"
            ));
        }
        let parameters = cursor.group('(', ')').ok_or_else(|| {
            format!("`{attribute}`: `fn {name}` has no parameter list the attribute can read")
        })?;
        Ok(Self {
            qualifiers,
            name,
            parameters: parameters.trim(),
            returns: return_type(&mut cursor),
        })
    }

    /// Whether the parameter list holds anything but whitespace and comments —
    /// `fn main(/* nothing */)` takes no arguments.
    pub fn takes_arguments(&self) -> bool {
        let mut cursor = Cursor::new(self.parameters);
        cursor.skip_space();
        !cursor.rest().is_empty()
    }
}

/// What `-> …` names, if anything: everything between the arrow and the body, minus a
/// `where` clause. `<`, `(` and `[` are followed through with [`Cursor::skip_atom`], so
/// a `Result<(), E>` or an `impl Trait + 'static` arrives whole and a `{` inside a
/// const-generic argument does not end the scan.
fn return_type<'a>(cursor: &mut Cursor<'a>) -> Option<&'a str> {
    cursor.skip_space();
    let rest = cursor.rest();
    let after_arrow = rest.strip_prefix("->")?;
    let mut depth = 0usize;
    let mut scan = Cursor::new(after_arrow);
    let mut end = after_arrow.len();
    loop {
        scan.skip_space();
        match scan.peek() {
            None => break,
            Some('<' | '(' | '[') => depth += 1,
            Some('>' | ')' | ']') => depth = depth.saturating_sub(1),
            Some('{') if depth == 0 => {
                end = after_arrow.len() - scan.rest().len();
                break;
            }
            Some(c) if depth == 0 && is_ident_start(c) => {
                let before = scan.rest().len();
                if scan.ident() == "where" {
                    end = after_arrow.len() - before;
                    break;
                }
                continue;
            }
            Some(_) => {}
        }
        scan.skip_atom();
    }
    Some(after_arrow[..end].trim()).filter(|t| !t.is_empty())
}

fn is_ident_start(c: char) -> bool {
    c == '_' || c.is_alphabetic()
}

/// `extern "C"` and `pub(crate)` carry a token of their own before the next word.
fn skip_qualifier_tail(cursor: &mut Cursor<'_>, word: &str) {
    cursor.skip_space();
    match (word, cursor.peek()) {
        ("extern", Some('"')) => cursor.skip_atom(),
        ("pub", Some('(')) => {
            let _ = cursor.group('(', ')');
        }
        _ => {}
    }
}

fn not_a_function(attribute: &str, word: &str, rest: &str) -> String {
    let found = if !word.is_empty() {
        format!("`{word}`")
    } else {
        match rest.chars().next() {
            Some(c) => format!("`{c}`"),
            None => "nothing".to_string(),
        }
    };
    format!("`{attribute}` can only be applied to a function: expected `fn`, found {found}")
}
