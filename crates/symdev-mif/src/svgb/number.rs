//! One SVG number, read the way `svgtbinencode` reads it (spec §4.4): single
//! precision, CSS unit suffix ignored, then **quantised to 16.16 fixed point**
//! — versions 1 and 4 write that quantised value back out as an IEEE-754
//! single, so `x="0.1"` is the float `6553/65536`, not `0.1` (experiment 59).

use symdev_core::{Error, Result};

/// A number value. The arithmetic behind it is double precision — that is what
/// makes relative path coordinates land on the tool's bytes — but every source
/// literal passes through `f32` first.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Number(pub f64);

impl Number {
    /// `|v| > 32765` makes the encoder drop the whole attribute (spec §4.4).
    pub const ATTRIBUTE_MAX: f64 = 32765.0;
    /// Unit suffixes the tool parses and then throws away (spec §4.4).
    const UNITS: [&'static str; 9] = ["px", "in", "cm", "mm", "pt", "pc", "em", "ex", "%"];

    /// An attribute value: a number with an optional unit suffix.
    pub fn parse(text: &str) -> Result<Self> {
        let text = text.trim();
        if text.starts_with('+') {
            return Err(Error::Other(format!(
                "a leading `+` in `{text}`: svgtbinencode drops the attribute \
                 or writes 0 for it; write the number without the sign"
            )));
        }
        let digits = Self::numeric_prefix(text);
        let unit = &text[digits.len()..];
        if !unit.is_empty() && !Self::UNITS.contains(&unit) {
            return Err(Error::Other(format!(
                "TODO: unit `{unit}` in `{text}` (not observed from svgtbinencode)"
            )));
        }
        let value: f32 = digits
            .parse()
            .map_err(|_| Error::Other(format!("not a number: {text}")))?;
        Ok(Self(f64::from(value)))
    }

    /// `true` when the number stays inside the range the tool accepts for an
    /// attribute; outside it the attribute is dropped altogether.
    pub fn in_attribute_range(self) -> bool {
        self.0.abs() <= Self::ATTRIBUTE_MAX
    }

    /// Clamped to 0…1, as the opacity attributes are (spec §4.6).
    pub fn clamped_opacity(self) -> Self {
        Self(self.0.clamp(0.0, 1.0))
    }

    /// The value times 65536, truncated toward zero: the tool's own internal
    /// representation, which path arithmetic is carried out in.
    pub fn fixed(self) -> i64 {
        (self.0 * 65536.0) as i64
    }

    /// Back from that representation; the division is exact.
    pub fn from_fixed(fixed: i64) -> Self {
        Self(fixed as f64 / 65536.0)
    }

    /// The four bytes of the value. Every version goes through 16.16 first;
    /// versions 1 and 4 then write the quantised number as a float.
    pub fn bytes(self, version: u8) -> [u8; 4] {
        let fixed = self.fixed().clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32;
        match version {
            2 | 3 => fixed.to_le_bytes(),
            _ => (Self::to_single(fixed) / 65536.0).to_le_bytes(),
        }
    }

    /// The tool's fixed-point-to-float conversion drops the bits that do not
    /// fit a `float` instead of rounding them: `0x7FFFFFFF` comes out as
    /// 32767.998046875, not 32768 (experiment 59).
    fn to_single(fixed: i32) -> f32 {
        let magnitude = fixed.unsigned_abs();
        let bits = u32::BITS - magnitude.leading_zeros();
        let kept = match bits.checked_sub(f32::MANTISSA_DIGITS) {
            Some(extra) if extra > 0 => (magnitude >> extra) << extra,
            _ => magnitude,
        };
        let value = kept as f32;
        if fixed < 0 { -value } else { value }
    }

    /// The longest prefix that parses as a number, so the unit can be checked.
    fn numeric_prefix(text: &str) -> &str {
        let bytes = text.as_bytes();
        let mut at = 0;
        if matches!(bytes.first(), Some(b'+' | b'-')) {
            at = 1;
        }
        while bytes.get(at).is_some_and(u8::is_ascii_digit) {
            at += 1;
        }
        if bytes.get(at) == Some(&b'.') {
            at += 1;
            while bytes.get(at).is_some_and(u8::is_ascii_digit) {
                at += 1;
            }
        }
        if matches!(bytes.get(at), Some(b'e' | b'E')) {
            let mut exponent = at + 1;
            if matches!(bytes.get(exponent), Some(b'+' | b'-')) {
                exponent += 1;
            }
            let start = exponent;
            while bytes.get(exponent).is_some_and(u8::is_ascii_digit) {
                exponent += 1;
            }
            if exponent > start {
                at = exponent;
            }
        }
        &text[..at]
    }
}

/// A whitespace- or comma-separated list of numbers, as `stroke-dasharray`,
/// `viewBox` and the `x`/`y` lists of `<text>` are written.
pub fn number_list(text: &str) -> Result<Vec<Number>> {
    text.split([' ', ',', '\t', '\r', '\n'])
        .filter(|s| !s.is_empty())
        .map(Number::parse)
        .collect()
}
