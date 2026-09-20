//! Colours and paint values (spec §4.6).

use symdev_core::{Error, Result};

use super::colours::KEYWORDS;

/// What a paint attribute may hold. `fill` carries a leading flag byte that
/// tells the two apart; `stroke`, `color` and `stop-color` are always a colour.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Paint {
    Colour(u32),
    /// `url(#id)`, stored without the `#`.
    Reference(String),
}

impl Paint {
    pub fn parse(text: &str) -> Result<Self> {
        let text = text.trim();
        if let Some(rest) = text.strip_prefix("url(") {
            let id = rest
                .trim_end_matches(')')
                .trim()
                .trim_matches(['"', '\''])
                .trim_start_matches('#');
            return Ok(Self::Reference(id.to_string()));
        }
        Ok(Self::Colour(colour(text)?))
    }
}

/// One `0x00RRGGBB` word, or the two paint keywords that live in the top byte.
pub fn colour(text: &str) -> Result<u32> {
    let text = text.trim();
    match text {
        "none" => return Ok(0x01ff_ffff),
        "currentColor" => return Ok(0x02ff_ffff),
        _ => {}
    }
    if let Some(hex) = text.strip_prefix('#') {
        return hex_colour(text, hex);
    }
    if let Some(list) = text.strip_prefix("rgb(").and_then(|v| v.strip_suffix(')')) {
        return rgb_colour(text, list);
    }
    let lower = text.to_ascii_lowercase();
    KEYWORDS
        .iter()
        .find(|(name, _)| *name == lower)
        .map(|(_, word)| *word)
        .ok_or_else(|| {
            Error::Other(format!(
                "TODO: colour `{text}` is not in svgtbinencode's keyword table \
                 (it would silently become black); write it as #rrggbb"
            ))
        })
}

/// `#rrggbb`, or `#rgb` expanded the way the tool expands it — each digit
/// becomes `(d << 4) | 0x0f`, not `d * 0x11` (spec §4.6).
fn hex_colour(text: &str, hex: &str) -> Result<u32> {
    let digits: Result<Vec<u32>> = hex
        .chars()
        .map(|c| {
            c.to_digit(16)
                .ok_or_else(|| Error::Other(format!("bad colour {text}")))
        })
        .collect();
    match digits?[..] {
        [r, g, b] => Ok((((r << 4) | 0xf) << 16) | (((g << 4) | 0xf) << 8) | ((b << 4) | 0xf)),
        [r1, r0, g1, g0, b1, b0] => {
            Ok((r1 << 20) | (r0 << 16) | (g1 << 12) | (g0 << 8) | (b1 << 4) | b0)
        }
        _ => Err(Error::Other(format!("bad colour {text}"))),
    }
}

/// `rgb(r,g,b)` with plain components or percentages. A percentage is scaled
/// by the `float` constant 2.55, which is a shade under 255/100, so 100 %
/// becomes 254 and 50 % becomes 127 (experiment 59).
fn rgb_colour(text: &str, list: &str) -> Result<u32> {
    let parts: Vec<&str> = list.split(',').map(str::trim).collect();
    if parts.len() != 3 {
        return Err(Error::Other(format!("bad colour {text}")));
    }
    let mut word = 0u32;
    for part in parts {
        let component = match part.strip_suffix('%') {
            Some(p) => {
                let percent: f32 = p
                    .parse()
                    .map_err(|_| Error::Other(format!("bad colour {text}")))?;
                ((f64::from(percent) * f64::from(2.55f32)).min(255.0) as i32) as u32
            }
            None => part
                .parse()
                .map_err(|_| Error::Other(format!("bad colour {text}")))?,
        };
        word = (word << 8) | (component & 0xff);
    }
    Ok(word)
}

/// The four bytes of a colour word: `0x00RRGGBB` at versions 3 and 4, byte
/// reversed to `0x00BBGGRR` at versions 1 and 2.
pub fn colour_bytes(word: u32, version: u8) -> [u8; 4] {
    match version {
        1 | 2 => [
            (word >> 16) as u8,
            (word >> 8) as u8,
            word as u8,
            (word >> 24) as u8,
        ],
        _ => word.to_le_bytes(),
    }
}
