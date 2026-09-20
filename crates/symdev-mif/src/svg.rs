//! Just enough XML to read an SVG Tiny icon: elements, attributes in document
//! order, comments and declarations skipped.

use symdev_core::{Error, Result};

/// One element of the source document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SvgElement {
    pub name: String,
    /// Attributes in document order, as `svgtbinencode` writes them.
    pub attributes: Vec<(String, String)>,
    pub children: Vec<SvgElement>,
}

impl SvgElement {
    pub fn attribute(&self, name: &str) -> Option<&str> {
        self.attributes
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.as_str())
    }

    /// The root element of an SVG document.
    pub fn parse(src: &str) -> Result<Self> {
        let mut p = SvgParser {
            src: src.as_bytes(),
            at: 0,
        };
        p.skip_prologue()?;
        let root = p.element()?;
        Ok(root)
    }
}

struct SvgParser<'a> {
    src: &'a [u8],
    at: usize,
}

impl SvgParser<'_> {
    fn error(&self, what: &str) -> Error {
        let line = 1 + self.src[..self.at].iter().filter(|&&b| b == b'\n').count();
        Error::Other(format!("SVG line {line}: {what}"))
    }

    fn space(&mut self) {
        while self
            .src
            .get(self.at)
            .is_some_and(|b| b.is_ascii_whitespace())
        {
            self.at += 1;
        }
    }

    /// `<?xml …?>`, comments and doctypes before the root element.
    fn skip_prologue(&mut self) -> Result<()> {
        loop {
            self.space();
            match self.src.get(self.at..self.at + 2) {
                Some(b"<?") => self.skip_to(b"?>")?,
                Some(b"<!") => {
                    if self.src.get(self.at..self.at + 4) == Some(b"<!--") {
                        self.skip_to(b"-->")?;
                    } else {
                        self.skip_to(b">")?;
                    }
                }
                _ => return Ok(()),
            }
        }
    }

    fn skip_to(&mut self, end: &[u8]) -> Result<()> {
        let from = self.at;
        while self.at + end.len() <= self.src.len() {
            if &self.src[self.at..self.at + end.len()] == end {
                self.at += end.len();
                return Ok(());
            }
            self.at += 1;
        }
        self.at = from;
        Err(self.error(&format!("unterminated `{}`", String::from_utf8_lossy(end))))
    }

    fn name(&mut self) -> Result<String> {
        let start = self.at;
        while self
            .src
            .get(self.at)
            .is_some_and(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b':' | b'.'))
        {
            self.at += 1;
        }
        if start == self.at {
            return Err(self.error("expected a name"));
        }
        Ok(String::from_utf8_lossy(&self.src[start..self.at]).into_owned())
    }

    /// One element, starting at its `<`.
    fn element(&mut self) -> Result<SvgElement> {
        self.space();
        if self.src.get(self.at) != Some(&b'<') {
            return Err(self.error("expected an element"));
        }
        self.at += 1;
        let name = self.name()?;
        let mut attributes = Vec::new();
        loop {
            self.space();
            match self.src.get(self.at) {
                Some(b'/') => {
                    self.at += 1;
                    if self.src.get(self.at) != Some(&b'>') {
                        return Err(self.error("expected `/>`"));
                    }
                    self.at += 1;
                    return Ok(SvgElement {
                        name,
                        attributes,
                        children: Vec::new(),
                    });
                }
                Some(b'>') => {
                    self.at += 1;
                    break;
                }
                Some(_) => {
                    let attr = self.name()?;
                    self.space();
                    if self.src.get(self.at) != Some(&b'=') {
                        return Err(self.error(&format!("attribute {attr} without a value")));
                    }
                    self.at += 1;
                    self.space();
                    attributes.push((attr, self.quoted()?));
                }
                None => return Err(self.error("unterminated element")),
            }
        }
        let children = self.children(&name)?;
        Ok(SvgElement {
            name,
            attributes,
            children,
        })
    }

    fn quoted(&mut self) -> Result<String> {
        let quote = match self.src.get(self.at) {
            Some(&q @ (b'"' | b'\'')) => q,
            _ => return Err(self.error("expected a quoted value")),
        };
        self.at += 1;
        let start = self.at;
        while self.src.get(self.at).is_some_and(|&b| b != quote) {
            self.at += 1;
        }
        if self.at >= self.src.len() {
            return Err(self.error("unterminated attribute value"));
        }
        let text = String::from_utf8_lossy(&self.src[start..self.at]).into_owned();
        self.at += 1;
        Ok(entities(&text))
    }

    /// Children until the matching `</name>`; character data is not supported.
    fn children(&mut self, name: &str) -> Result<Vec<SvgElement>> {
        let mut children = Vec::new();
        loop {
            self.space();
            match self.src.get(self.at..self.at + 2) {
                None => return Err(self.error(&format!("unterminated <{name}>"))),
                Some(b"</") => {
                    self.at += 2;
                    let close = self.name()?;
                    if close != name {
                        return Err(self.error(&format!("</{close}> closes <{name}>")));
                    }
                    self.space();
                    if self.src.get(self.at) != Some(&b'>') {
                        return Err(self.error("expected `>`"));
                    }
                    self.at += 1;
                    return Ok(children);
                }
                Some(b"<!") => {
                    self.skip_to(b"-->")?;
                }
                Some(s) if s.starts_with(b"<") => children.push(self.element()?),
                _ => {
                    return Err(self.error(&format!(
                        "TODO: character data in <{name}> (the icon encoder takes shapes only)"
                    )));
                }
            }
        }
    }
}

/// The five XML entities; anything else stays as written.
fn entities(text: &str) -> String {
    text.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}

#[cfg(test)]
mod tests;
