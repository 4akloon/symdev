//! SVG Tiny → Symbian binary SVG (`.svgb`), as `svgtbinencode` writes it
//! ([svgb-mif-spec.md](../../../docs/research/svgb-mif-spec.md) §4).

use symdev_core::{Error, Result};

use crate::svg::SvgElement;

/// Encoder for one document. Version 3 (fixed point, `0x00RRGGBB`) is what
/// `mifconv` uses by default.
pub struct Svgb {
    pub version: u8,
    out: Vec<u8>,
}

impl Svgb {
    /// The version `mifconv` encodes with by default.
    pub const MIFCONV_VERSION: u8 = 3;
    const HEADER_TAIL: [u8; 3] = [0x56, 0xfa, 0x03];
    const END_OF_ATTRIBUTES: [u8; 2] = [0xe8, 0x03];
    const END_OF_ELEMENT: u8 = 0xfe;
    const END_OF_DOCUMENT: u8 = 0xff;
    /// `|v| > 32765.0` makes the encoder drop the attribute (spec §4.4).
    const MAX_NUMBER: f32 = 32765.0;

    /// Element name → token (spec §4.8), for the shapes an icon needs.
    fn element_token(name: &str) -> Option<u8> {
        Some(match name {
            "svg" => 0x00,
            "g" => 0x0b,
            "circle" => 0x1b,
            "rect" => 0x21,
            _ => return None,
        })
    }

    /// Attribute name → id (spec §4.9).
    fn attribute_id(name: &str) -> Option<u16> {
        Some(match name {
            "fill" => 0x0000,
            "width" => 0x001a,
            "height" => 0x001b,
            "r" => 0x001c,
            "rx" => 0x001d,
            "ry" => 0x001e,
            "cx" => 0x002e,
            "cy" => 0x002f,
            "y" => 0x0030,
            "x" => 0x0031,
            "viewBox" => 0x0058,
            "baseProfile" => 0x0059,
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
        let token = Self::element_token(&e.name).ok_or_else(|| {
            Error::Other(format!(
                "TODO: <{}> (the icon encoder takes svg, g, rect and circle)",
                e.name
            ))
        })?;
        self.out.push(token);
        for (name, value) in &e.attributes {
            self.attribute(&e.name, name, value)?;
        }
        self.out.extend_from_slice(&Self::END_OF_ATTRIBUTES);
        for child in &e.children {
            self.element(child)?;
        }
        self.out.push(Self::END_OF_ELEMENT);
        Ok(())
    }

    /// Namespace declarations carry no bytes; anything else we do not encode is
    /// an error rather than a silently missing part of the icon.
    fn attribute(&mut self, element: &str, name: &str, value: &str) -> Result<()> {
        if name == "xmlns" || name.starts_with("xmlns:") {
            return Ok(());
        }
        let Some(id) = Self::attribute_id(name) else {
            return Err(Error::Other(format!(
                "TODO: attribute {name} on <{element}> (not in the icon subset)"
            )));
        };
        let mut value_bytes = match name {
            "fill" => self.paint(value)?,
            "baseProfile" => Self::string(value)?,
            "viewBox" => {
                let mut out = Vec::new();
                let numbers: Vec<&str> =
                    value.split([' ', ',']).filter(|s| !s.is_empty()).collect();
                if numbers.len() != 4 {
                    return Err(Error::Other(format!("viewBox needs four numbers: {value}")));
                }
                for n in numbers {
                    match self.number(n)? {
                        Some(bytes) => out.extend_from_slice(&bytes),
                        None => return Ok(()),
                    }
                }
                out
            }
            _ => {
                let Some(bytes) = self.number(value)? else {
                    return Ok(());
                };
                let mut out = Vec::new();
                // `width`/`height` on <svg> carry a unit byte (spec §4.5).
                if element == "svg" && matches!(name, "width" | "height") {
                    out.push(u8::from(value.trim_end().ends_with('%')));
                }
                out.extend_from_slice(&bytes);
                out
            }
        };
        let mut record = id.to_le_bytes().to_vec();
        record.append(&mut value_bytes);
        self.out.extend_from_slice(&record);
        Ok(())
    }

    /// One number, or `None` when the value is out of range and the whole
    /// attribute is dropped (spec §4.4).
    fn number(&self, text: &str) -> Result<Option<[u8; 4]>> {
        let text = text.trim();
        let digits: String = text
            .chars()
            .take_while(|c| c.is_ascii_digit() || matches!(c, '.' | '-' | '+' | 'e' | 'E'))
            .collect();
        let value: f32 = digits
            .parse()
            .map_err(|_| Error::Other(format!("not a number: {text}")))?;
        if value.abs() > Self::MAX_NUMBER {
            return Ok(None);
        }
        Ok(Some(match self.version {
            2 | 3 => (((value as f64) * 65536.0) as i32).to_le_bytes(),
            _ => value.to_le_bytes(),
        }))
    }

    /// `fill`: a flag byte, then a colour word or a `url(#id)` string (spec §4.6).
    fn paint(&self, value: &str) -> Result<Vec<u8>> {
        let value = value.trim();
        if let Some(rest) = value.strip_prefix("url(") {
            let id = rest.trim_end_matches(')').trim_start_matches('#');
            let mut out = vec![1];
            out.extend_from_slice(&Self::string(id)?);
            return Ok(out);
        }
        let mut out = vec![0];
        out.extend_from_slice(&self.colour(value)?);
        Ok(out)
    }

    fn colour(&self, value: &str) -> Result<[u8; 4]> {
        let word = match value {
            "none" => 0x01ff_ffff,
            "currentColor" => 0x02ff_ffff,
            _ => Self::rgb(value)?,
        };
        Ok(match self.version {
            1 | 2 => [
                (word >> 16) as u8,
                (word >> 8) as u8,
                word as u8,
                (word >> 24) as u8,
            ],
            _ => word.to_le_bytes(),
        })
    }

    /// `#rrggbb`, `#rgb` (expanded as the tool does, `(d << 4) | 0x0f`), or
    /// `rgb(r,g,b)` with plain or percentage components.
    fn rgb(value: &str) -> Result<u32> {
        if let Some(hex) = value.strip_prefix('#') {
            let digits: Result<Vec<u32>> = hex
                .chars()
                .map(|c| {
                    c.to_digit(16)
                        .ok_or_else(|| Error::Other(format!("bad colour {value}")))
                })
                .collect();
            let digits = digits?;
            return match digits[..] {
                [r, g, b] => Ok(((r << 4 | 0xf) << 16) | ((g << 4 | 0xf) << 8) | (b << 4 | 0xf)),
                [r1, r0, g1, g0, b1, b0] => {
                    Ok((r1 << 20) | (r0 << 16) | (g1 << 12) | (g0 << 8) | (b1 << 4) | b0)
                }
                _ => Err(Error::Other(format!("bad colour {value}"))),
            };
        }
        if let Some(list) = value.strip_prefix("rgb(").and_then(|v| v.strip_suffix(')')) {
            let parts: Vec<&str> = list.split(',').map(str::trim).collect();
            if parts.len() != 3 {
                return Err(Error::Other(format!("bad colour {value}")));
            }
            let mut word = 0u32;
            for part in parts {
                let component = match part.strip_suffix('%') {
                    Some(p) => (p.parse::<f32>().unwrap_or(0.0) * 255.0 / 100.0) as u32,
                    None => part.parse::<u32>().unwrap_or(0),
                };
                word = (word << 8) | (component & 0xff);
            }
            return Ok(word);
        }
        Err(Error::Other(format!(
            "TODO: colour keyword `{value}` (write it as #rrggbb)"
        )))
    }

    /// Length byte, then UTF-16LE (spec §4.7). The tool truncates silently past
    /// 127 characters; we refuse instead.
    fn string(text: &str) -> Result<Vec<u8>> {
        let units: Vec<u16> = text.encode_utf16().collect();
        if units.len() > 127 {
            return Err(Error::Other(format!(
                "SVGB string longer than 127 characters: {text}"
            )));
        }
        let mut out = vec![(2 * units.len()) as u8];
        units
            .iter()
            .for_each(|u| out.extend_from_slice(&u.to_le_bytes()));
        Ok(out)
    }
}

#[cfg(test)]
mod tests;
