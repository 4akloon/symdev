//! SVG Tiny → Symbian binary SVG (`.svgb`), as `svgtbinencode` writes it
//! ([svgb-mif-spec.md](../../../docs/research/svgb-mif-spec.md) §4).

mod attribute;
mod colours;
mod number;
mod paint;
mod path;
mod transform;
mod value;

use symdev_core::{Error, Result};

use crate::svg::SvgElement;
use attribute::Layout;
use value::Value;

/// Encoder for one document. Version 3 (fixed point, `0x00RRGGBB`) is what
/// `mifconv` uses by default.
pub struct Svgb {
    pub version: u8,
    out: Vec<u8>,
    /// Every `id` written so far, in document order: `<use>` resolves its
    /// `xlink:href` against them (spec §4.9).
    ids: Vec<String>,
}

impl Svgb {
    /// The version `mifconv` encodes with by default.
    pub const MIFCONV_VERSION: u8 = 3;
    const HEADER_TAIL: [u8; 3] = [0x56, 0xfa, 0x03];
    const END_OF_ATTRIBUTES: [u8; 2] = [0xe8, 0x03];
    const CHARACTER_DATA: u8 = 0xfd;
    const END_OF_ELEMENT: u8 = 0xfe;
    const END_OF_DOCUMENT: u8 = 0xff;

    /// Element name → token (spec §4.8), for the shapes an icon is built from.
    fn element_token(name: &str) -> Option<u8> {
        Some(match name {
            "svg" => 0x00,
            "defs" => 0x03,
            "desc" => 0x04,
            "title" => 0x07,
            "g" => 0x0b,
            "switch" => 0x0f,
            "a" => 0x12,
            "image" => 0x16,
            "text" => 0x19,
            "use" => 0x1a,
            "circle" => 0x1b,
            "ellipse" => 0x1c,
            "line" => 0x1d,
            "path" => 0x1e,
            "polygon" => 0x1f,
            "polyline" => 0x20,
            "rect" => 0x21,
            "linearGradient" => 0x28,
            "radialGradient" => 0x29,
            "stop" => 0x2a,
            "solidColor" => 0x2e,
            _ => return None,
        })
    }

    pub fn new(version: u8) -> Result<Self> {
        if !(1..=4).contains(&version) {
            return Err(Error::Other(format!("SVGB version {version} (1-4)")));
        }
        Ok(Self {
            version,
            out: Vec::new(),
            ids: Vec::new(),
        })
    }

    pub fn encode(mut self, root: &SvgElement) -> Result<Vec<u8>> {
        self.out.push(0xcb + self.version);
        self.out.extend_from_slice(&Self::HEADER_TAIL);
        self.element(root)?;
        self.out.push(Self::END_OF_DOCUMENT);
        Ok(self.out)
    }

    fn element(&mut self, e: &SvgElement) -> Result<()> {
        // `<metadata>` and everything in it is dropped, exactly as the tool
        // drops it; any other unknown element is an error, because dropping it
        // would silently lose part of the picture.
        if e.name == "metadata" {
            return Ok(());
        }
        let token = Self::element_token(&e.name).ok_or_else(|| {
            Error::Other(format!(
                "TODO: <{}> (not in the icon subset svgtbinencode was pinned down for)",
                e.name
            ))
        })?;
        self.out.push(token);
        for (name, value) in &e.attributes {
            self.attribute(&e.name, name, value)?;
        }
        self.out.extend_from_slice(&Self::END_OF_ATTRIBUTES);
        // Only `<text>` carries its character data into the file (spec §4.12).
        if e.name == "text" {
            self.out.push(Self::CHARACTER_DATA);
            self.out.extend_from_slice(&Value::string(&e.text())?);
        }
        for child in &e.children {
            self.element(child)?;
        }
        self.out.push(Self::END_OF_ELEMENT);
        Ok(())
    }

    /// Namespace declarations and the attributes in `attribute::dropped` carry
    /// no bytes; anything else we cannot encode is an error rather than a
    /// silently missing part of the icon.
    fn attribute(&mut self, element: &str, name: &str, value: &str) -> Result<()> {
        if name == "xmlns" || name.starts_with("xmlns:") || attribute::dropped(name) {
            return Ok(());
        }
        if name == "style" {
            return self.style(element, value);
        }
        let Some((id, layout)) = attribute::lookup(name) else {
            return Err(Error::Other(format!(
                "TODO: attribute {name} on <{element}> (not in the icon subset)"
            )));
        };
        self.record(element, name, id, layout, value)?;
        if name == "id" {
            self.ids.push(value.to_string());
        }
        Ok(())
    }

    /// `style` is parsed into the same records the properties would give as
    /// attributes, at the position the `style` attribute itself occupies.
    fn style(&mut self, element: &str, value: &str) -> Result<()> {
        for declaration in value.split(';') {
            let declaration = declaration.trim();
            if declaration.is_empty() {
                continue;
            }
            let Some((name, property)) = declaration.split_once(':') else {
                return Err(Error::Other(format!(
                    "style declaration without a `:`: {declaration}"
                )));
            };
            let (name, property) = (name.trim(), property.trim());
            match attribute::lookup(name) {
                Some((id, layout)) if attribute::style_property(name) => {
                    self.record(element, name, id, layout, property)?;
                }
                Some(_) => {
                    return Err(Error::Other(format!(
                        "TODO: `{name}` inside style= — svgtbinencode's style parser \
                         mishandles it; write it as an attribute"
                    )));
                }
                // A property the tool does not know carries no bytes.
                None => {}
            }
        }
        Ok(())
    }

    /// One attribute record, unless the value is one the tool drops.
    fn record(
        &mut self,
        element: &str,
        name: &str,
        id: u16,
        layout: Layout,
        value: &str,
    ) -> Result<()> {
        let encoder = Value {
            version: self.version,
        };
        let Some(bytes) = encoder.bytes(element, name, layout, value)? else {
            return Ok(());
        };
        self.out.extend_from_slice(&id.to_le_bytes());
        self.out.extend_from_slice(&bytes);
        if layout == Layout::Href && element == "use" {
            // `<use>` writes a second string after the reference: the target
            // id when an element already carries it, else the reference again
            // (spec §4.9, refined by experiment 59).
            self.out
                .extend_from_slice(&Value::string(&self.target(value))?);
        }
        Ok(())
    }

    /// The fragment of `href` when an `id` seen so far matches it.
    fn target(&self, href: &str) -> String {
        match href.strip_prefix('#') {
            Some(id) if self.ids.iter().any(|seen| seen == id) => id.to_string(),
            _ => href.to_string(),
        }
    }
}

#[cfg(test)]
mod tests;
