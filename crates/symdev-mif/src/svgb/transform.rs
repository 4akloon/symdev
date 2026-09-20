//! The `transform` attribute: one 2×3 matrix and a kind word (spec §4.11).

use symdev_core::{Error, Result};

use super::number::Number;

/// The product of the transform list, plus the kind word the tool derives from
/// the functions it saw.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform {
    /// `a b c d e f`, the SVG order.
    matrix: [f64; 6],
    kind: u32,
}

impl Transform {
    const TRANSLATE: u32 = 1;
    const SCALE: u32 = 2;
    const ROTATE: u32 = 4;

    /// The six numbers in the order the file wants them: `a c e` then `b d f`.
    pub fn numbers(&self) -> [Number; 6] {
        let [a, b, c, d, e, f] = self.matrix;
        [a, c, e, b, d, f].map(Number)
    }

    pub fn kind(&self) -> u32 {
        self.kind
    }

    pub fn parse(text: &str) -> Result<Self> {
        let mut result = Self {
            matrix: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
            kind: 0,
        };
        let mut rest = text.trim();
        if rest.is_empty() {
            return Err(Error::Other("empty transform".into()));
        }
        while !rest.is_empty() {
            let (name, args, tail) = Self::function(rest)?;
            let (matrix, kind) = Self::function_matrix(name, &args)?;
            result.matrix = Self::multiply(result.matrix, matrix);
            result.kind |= kind;
            rest = tail.trim_start_matches([' ', ',', '\t', '\r', '\n']);
        }
        Ok(result)
    }

    /// `name(args)` and whatever follows it.
    fn function(text: &str) -> Result<(&str, Vec<f64>, &str)> {
        let open = text
            .find('(')
            .ok_or_else(|| Error::Other(format!("transform without `(`: {text}")))?;
        let close = text
            .find(')')
            .ok_or_else(|| Error::Other(format!("transform without `)`: {text}")))?;
        if close < open {
            return Err(Error::Other(format!("malformed transform: {text}")));
        }
        let args: Result<Vec<f64>> = text[open + 1..close]
            .split([' ', ',', '\t', '\r', '\n'])
            .filter(|s| !s.is_empty())
            .map(|s| Number::parse(s).map(|n| n.0))
            .collect();
        Ok((text[..open].trim(), args?, &text[close + 1..]))
    }

    /// One function's own matrix and the bits it contributes to the kind word.
    /// `rotate`, `skewX` and `skewY` are refused: the tool's sine rounding is
    /// off by up to 2/65536 and could not be reproduced (spec §4.11).
    fn function_matrix(name: &str, args: &[f64]) -> Result<([f64; 6], u32)> {
        Ok(match (name, args.len()) {
            ("translate", 1) => ([1.0, 0.0, 0.0, 1.0, args[0], 0.0], Self::TRANSLATE),
            ("translate", 2) => ([1.0, 0.0, 0.0, 1.0, args[0], args[1]], Self::TRANSLATE),
            ("scale", 1) => ([args[0], 0.0, 0.0, args[0], 0.0, 0.0], Self::SCALE),
            ("scale", 2) => ([args[0], 0.0, 0.0, args[1], 0.0, 0.0], Self::SCALE),
            ("matrix", 6) => {
                let m = [args[0], args[1], args[2], args[3], args[4], args[5]];
                (m, Self::matrix_kind(m))
            }
            ("rotate" | "skewX" | "skewY", _) => {
                return Err(Error::Other(format!(
                    "TODO: transform {name}() — svgtbinencode's sine differs from \
                     a double-precision one by up to 2/65536 and was not reproduced; \
                     bake the rotation into matrix(a,b,c,d,e,f)"
                )));
            }
            ("translate" | "scale", n) => {
                return Err(Error::Other(format!(
                    "transform {name}() with {n} arguments"
                )));
            }
            _ => {
                return Err(Error::Other(format!(
                    "TODO: transform {name}() (not observed from svgtbinencode)"
                )));
            }
        })
    }

    /// `matrix()` is the one function whose kind comes from its values: the
    /// identity contributes nothing, anything else sets the scale bit, plus
    /// the translate bit for `e`/`f` and the rotate bit for `b`/`c`.
    fn matrix_kind(m: [f64; 6]) -> u32 {
        if m == [1.0, 0.0, 0.0, 1.0, 0.0, 0.0] {
            return 0;
        }
        let mut kind = Self::SCALE;
        if m[4] != 0.0 || m[5] != 0.0 {
            kind |= Self::TRANSLATE;
        }
        if m[1] != 0.0 || m[2] != 0.0 {
            kind |= Self::ROTATE;
        }
        kind
    }

    /// `left · right`, both `a b c d e f`.
    fn multiply(l: [f64; 6], r: [f64; 6]) -> [f64; 6] {
        [
            l[0] * r[0] + l[2] * r[1],
            l[1] * r[0] + l[3] * r[1],
            l[0] * r[2] + l[2] * r[3],
            l[1] * r[2] + l[3] * r[3],
            l[0] * r[4] + l[2] * r[5] + l[4],
            l[1] * r[4] + l[3] * r[5] + l[5],
        ]
    }
}
