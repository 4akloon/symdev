//! The bytes that follow an attribute id, one layout at a time (spec §4.9).

use symdev_core::{Error, Result};

use super::attribute::{self, Layout};
use super::number::{self, Number};
use super::paint::{self, Paint};
use super::path::PathData;
use super::transform::Transform;

/// One `.svgb` version's view of attribute values.
pub struct Value {
    pub version: u8,
}

impl Value {
    /// The tool writes `(2n) mod 256` and throws the rest away; we refuse.
    const MAX_STRING: usize = 127;

    /// The value bytes, or `None` when the tool drops the whole attribute.
    pub fn bytes(
        &self,
        element: &str,
        name: &str,
        layout: Layout,
        value: &str,
    ) -> Result<Option<Vec<u8>>> {
        Ok(match layout {
            // `x` and `y` are lists on `<text>` and plain numbers elsewhere.
            Layout::Number if element == "text" && matches!(name, "x" | "y") => {
                self.number_list(value)?
            }
            Layout::Number => return self.number(element, name, value),
            Layout::Opacity => Some(self.opacity(name, value)?),
            Layout::Colour => {
                Some(paint::colour_bytes(paint::colour(value)?, self.version).to_vec())
            }
            Layout::Fill => Some(self.fill(value)?),
            Layout::Text | Layout::Href => Some(Self::string(value)?),
            Layout::TextList => Some(Self::string_list(value)?),
            Layout::NumberList => self.number_list(value)?,
            Layout::ViewBox => self.view_box(value)?,
            Layout::PathData => Some(self.path_data(element, name, value)?),
            Layout::Transform => Some(self.transform(value)?),
            Layout::Enum32 => Some(Self::keyword(name, value)?.to_le_bytes().to_vec()),
            Layout::Enum8 => Some(vec![Self::keyword(name, value)? as u8]),
            Layout::PreserveAspectRatio => self.aspect_ratio(element, value)?,
        })
    }

    /// An opacity, clamped to 0…1. A value written without its leading zero
    /// is refused: the tool's opacity parser cannot read `.25` and silently
    /// substitutes 1 (0 for `stop-opacity`), which experiment 59 confirmed.
    fn opacity(&self, name: &str, value: &str) -> Result<Vec<u8>> {
        if value.trim_start().starts_with('.') {
            return Err(Error::Other(format!(
                "{name}=\"{value}\": svgtbinencode reads an opacity without its \
                 leading zero as 1; write it as 0{}",
                value.trim()
            )));
        }
        Ok(Number::parse(value)?
            .clamped_opacity()
            .bytes(self.version)
            .to_vec())
    }

    /// `preserveAspectRatio` is two bytes on `<svg>`, and only for `none`;
    /// on `<image>` it is an ordinary string. Anywhere else the tool writes a
    /// truncated file, so it is refused (experiment 59).
    fn aspect_ratio(&self, element: &str, value: &str) -> Result<Option<Vec<u8>>> {
        match element {
            "svg" => Ok((value.trim() == "none").then(|| vec![0x00, 0x02])),
            "image" => Ok(Some(Self::string(value)?)),
            _ => Err(Error::Other(format!(
                "TODO: preserveAspectRatio on <{element}> — svgtbinencode leaves a \
                 truncated file behind; it is only usable on <svg> and <image>"
            ))),
        }
    }

    /// A plain number, with the `<svg>` unit byte and the two drop rules.
    fn number(&self, element: &str, name: &str, value: &str) -> Result<Option<Vec<u8>>> {
        let parsed = Number::parse(value)?;
        if !parsed.in_attribute_range() {
            return Ok(None);
        }
        // SVG forbids a negative `width`/`height`, and the tool drops it.
        if matches!(name, "width" | "height") && parsed.0 < 0.0 {
            return Ok(None);
        }
        let mut out = Vec::new();
        if element == "svg" && matches!(name, "width" | "height") {
            out.push(u8::from(value.trim_end().ends_with('%')));
        }
        out.extend_from_slice(&parsed.bytes(self.version));
        Ok(Some(out))
    }

    /// A one-byte count and that many numbers.
    fn number_list(&self, value: &str) -> Result<Option<Vec<u8>>> {
        let numbers = number::number_list(value)?;
        if numbers.iter().any(|n| !n.in_attribute_range()) {
            return Ok(None);
        }
        let mut out = vec![
            u8::try_from(numbers.len())
                .map_err(|_| Error::Other(format!("more than 255 numbers in `{value}`")))?,
        ];
        self.append(&mut out, &numbers);
        Ok(Some(out))
    }

    /// Four numbers separated by whitespace only: a comma anywhere in the
    /// value makes the tool drop the attribute (experiment 59).
    fn view_box(&self, value: &str) -> Result<Option<Vec<u8>>> {
        if value.contains(',') {
            return Err(Error::Other(format!(
                "viewBox=\"{value}\": svgtbinencode drops a viewBox that has a \
                 comma in it; separate the four numbers with spaces"
            )));
        }
        let numbers = number::number_list(value)?;
        if numbers.len() != 4 {
            return Err(Error::Other(format!("viewBox needs four numbers: {value}")));
        }
        if numbers.iter().any(|n| !n.in_attribute_range()) {
            return Ok(None);
        }
        let mut out = Vec::new();
        self.append(&mut out, &numbers);
        Ok(Some(out))
    }

    /// `fill`: a flag byte, then a colour word or a `url(#id)` string.
    fn fill(&self, value: &str) -> Result<Vec<u8>> {
        Ok(match Paint::parse(value)? {
            Paint::Colour(word) => {
                let mut out = vec![0];
                out.extend_from_slice(&paint::colour_bytes(word, self.version));
                out
            }
            Paint::Reference(id) => {
                let mut out = vec![1];
                out.extend_from_slice(&Self::string(&id)?);
                out
            }
        })
    }

    /// Command count, commands, value count, values — the same payload for
    /// `d` and for `points` (spec §4.10). Path values are not range checked:
    /// the tool lets the fixed-point conversion saturate.
    fn path_data(&self, element: &str, name: &str, value: &str) -> Result<Vec<u8>> {
        let data = match name {
            "d" => PathData::path(value)?,
            _ => PathData::points(value, element == "polygon")?,
        };
        let commands = u16::try_from(data.commands().len())
            .map_err(|_| Error::Other(format!("more than 65535 path commands in `{name}`")))?;
        let count = u16::try_from(data.values().len())
            .map_err(|_| Error::Other(format!("more than 65535 path values in `{name}`")))?;
        let mut out = commands.to_le_bytes().to_vec();
        out.extend_from_slice(data.commands());
        out.extend_from_slice(&count.to_le_bytes());
        self.append(&mut out, data.values());
        Ok(out)
    }

    fn transform(&self, value: &str) -> Result<Vec<u8>> {
        let parsed = Transform::parse(value)?;
        let mut out = Vec::new();
        self.append(&mut out, &parsed.numbers());
        out.extend_from_slice(&parsed.kind().to_le_bytes());
        Ok(out)
    }

    fn append(&self, out: &mut Vec<u8>, numbers: &[Number]) {
        numbers
            .iter()
            .for_each(|n| out.extend_from_slice(&n.bytes(self.version)));
    }

    fn keyword(name: &str, value: &str) -> Result<u32> {
        attribute::keyword(name, value.trim()).ok_or_else(|| {
            Error::Other(format!(
                "TODO: {name}=\"{value}\" (svgtbinencode's code for it was not observed)"
            ))
        })
    }

    /// Length byte, then UTF-16LE (spec §4.7). The tool truncates silently
    /// past 127 characters; we refuse instead.
    pub fn string(text: &str) -> Result<Vec<u8>> {
        let units: Vec<u16> = text.encode_utf16().collect();
        if units.len() > Self::MAX_STRING {
            return Err(Error::Other(format!(
                "SVGB string longer than {} characters: {text}",
                Self::MAX_STRING
            )));
        }
        let mut out = vec![(2 * units.len()) as u8];
        units
            .iter()
            .for_each(|u| out.extend_from_slice(&u.to_le_bytes()));
        Ok(out)
    }

    /// A one-byte item count followed by that many strings.
    fn string_list(value: &str) -> Result<Vec<u8>> {
        let items: Vec<&str> = value.split_whitespace().collect();
        let mut out = vec![
            u8::try_from(items.len())
                .map_err(|_| Error::Other(format!("more than 255 items in `{value}`")))?,
        ];
        for item in items {
            out.extend_from_slice(&Self::string(item)?);
        }
        Ok(out)
    }
}
