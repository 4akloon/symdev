//! `d` and `points`: the reduced command list the tool writes (spec §4.10).
//!
//! Everything becomes absolute coordinates and one of five commands — move,
//! line, quadratic, cubic, close. Two deviations from SVG that the tool makes
//! and this encoder copies: a repeated coordinate pair after `M`/`m` stays a
//! *move* instead of becoming a line, and the point a smooth curve reflects
//! is kept in one variable that every command except `M` and `Z` updates —
//! so `S` happily reflects a quadratic control point, a plain `L` leaves
//! behind `2·new − old` for the next `S`, and a `Z` leaves the previous
//! curve's reflection untouched (experiment 59).
//!
//! All of the arithmetic is done in 16.16 fixed point, because the tool does
//! it there: every literal is truncated toward zero on the way in and relative
//! coordinates are then added as integers, so `84` then `-8.059` lands on
//! `4976870`, one more than the double-precision `75.941` would give
//! (experiment 59).

use symdev_core::{Error, Result};

use super::number::Number;

/// Command codes, as written one byte each.
const MOVE: u8 = 0;
const LINE: u8 = 1;
const QUAD: u8 = 2;
const CUBIC: u8 = 3;
const CLOSE: u8 = 4;

/// A coordinate in 16.16 fixed point, wide enough that the saturation only
/// happens when the four bytes are written.
type Fixed = i64;

/// A parsed path payload.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PathData {
    commands: Vec<u8>,
    values: Vec<Number>,
}

impl PathData {
    pub fn commands(&self) -> &[u8] {
        &self.commands
    }

    pub fn values(&self) -> &[Number] {
        &self.values
    }

    /// The `points` list of `<polyline>` (`close` false) or `<polygon>`.
    pub fn points(text: &str, close: bool) -> Result<Self> {
        let numbers = super::number::number_list(text)?;
        if numbers.is_empty() {
            return Ok(Self::default());
        }
        if numbers.len() % 2 != 0 {
            return Err(Error::Other(format!(
                "points needs an even number of coordinates, got {}: {text}",
                numbers.len()
            )));
        }
        let mut data = Self {
            commands: Vec::new(),
            values: numbers,
        };
        data.commands.push(MOVE);
        for _ in 1..data.values.len() / 2 {
            data.commands.push(LINE);
        }
        if close {
            data.commands.push(CLOSE);
        }
        Ok(data)
    }

    /// The `d` attribute.
    pub fn path(text: &str) -> Result<Self> {
        let mut scanner = PathScanner::new(text);
        scanner.run()?;
        Ok(scanner.data)
    }

    fn emit(&mut self, command: u8, points: &[(Fixed, Fixed)]) {
        self.commands.push(command);
        for &(x, y) in points {
            self.values.push(Number::from_fixed(x));
            self.values.push(Number::from_fixed(y));
        }
    }
}

struct PathScanner<'a> {
    src: &'a [u8],
    at: usize,
    data: PathData,
    /// Current point, subpath start, and the control point the next `S`/`T`
    /// uses. The reflection point starts at the origin and survives `M`
    /// and `Z` untouched.
    point: (Fixed, Fixed),
    start: (Fixed, Fixed),
    reflection: (Fixed, Fixed),
}

impl<'a> PathScanner<'a> {
    fn new(text: &'a str) -> Self {
        Self {
            src: text.as_bytes(),
            at: 0,
            data: PathData::default(),
            point: (0, 0),
            start: (0, 0),
            reflection: (0, 0),
        }
    }

    fn run(&mut self) -> Result<()> {
        let mut command = 0u8;
        loop {
            self.separators();
            let Some(&byte) = self.src.get(self.at) else {
                return Ok(());
            };
            if byte.is_ascii_alphabetic() {
                command = byte;
                self.at += 1;
            } else if command == 0 {
                return Err(self.error("path data must start with a command"));
            } else if command.eq_ignore_ascii_case(&b'Z') {
                return Err(self.error("coordinates after a close-path command"));
            }
            // An implicit repeat keeps the command it repeats, so a second
            // coordinate pair after `M`/`m` stays a move; SVG would make it a
            // line, but the tool writes another move.
            self.command(command)?;
        }
    }

    fn command(&mut self, command: u8) -> Result<()> {
        let relative = command.is_ascii_lowercase();
        let (x, y) = self.point;
        let origin = if relative { (x, y) } else { (0, 0) };
        match command.to_ascii_uppercase() {
            b'M' => {
                let p = self.point_at(origin)?;
                self.data.emit(MOVE, &[p]);
                self.point = p;
                self.start = p;
            }
            b'L' => {
                let p = self.point_at(origin)?;
                self.line(p);
            }
            b'H' => {
                let p = (origin.0 + self.number()?, y);
                self.line(p);
            }
            b'V' => {
                let p = (x, origin.1 + self.number()?);
                self.line(p);
            }
            b'C' => {
                let c1 = self.point_at(origin)?;
                let c2 = self.point_at(origin)?;
                let p = self.point_at(origin)?;
                self.cubic(c1, c2, p);
            }
            b'S' => {
                let c1 = self.reflection;
                let c2 = self.point_at(origin)?;
                let p = self.point_at(origin)?;
                self.cubic(c1, c2, p);
            }
            b'Q' => {
                let c = self.point_at(origin)?;
                let p = self.point_at(origin)?;
                self.quad(c, p);
            }
            b'T' => {
                let c = self.reflection;
                let p = self.point_at(origin)?;
                self.quad(c, p);
            }
            b'Z' => {
                self.data.emit(CLOSE, &[]);
                self.point = self.start;
            }
            b'A' => {
                return Err(self.error(
                    "elliptical arcs are not supported by svgtbinencode — it \
                     silently empties the whole path; replace the arc with C curves",
                ));
            }
            other => {
                return Err(self.error(&format!(
                    "unknown path command `{}` (svgtbinencode empties the whole path)",
                    other as char
                )));
            }
        }
        Ok(())
    }

    fn line(&mut self, p: (Fixed, Fixed)) {
        self.data.emit(LINE, &[p]);
        self.reflection = reflect(p, self.point);
        self.point = p;
    }

    fn cubic(&mut self, c1: (Fixed, Fixed), c2: (Fixed, Fixed), p: (Fixed, Fixed)) {
        self.data.emit(CUBIC, &[c1, c2, p]);
        self.reflection = reflect(p, c2);
        self.point = p;
    }

    fn quad(&mut self, c: (Fixed, Fixed), p: (Fixed, Fixed)) {
        self.data.emit(QUAD, &[c, p]);
        self.reflection = reflect(p, c);
        self.point = p;
    }

    fn point_at(&mut self, origin: (Fixed, Fixed)) -> Result<(Fixed, Fixed)> {
        let x = origin.0 + self.number()?;
        let y = origin.1 + self.number()?;
        Ok((x, y))
    }

    fn separators(&mut self) {
        while self
            .src
            .get(self.at)
            .is_some_and(|b| b.is_ascii_whitespace() || *b == b',')
        {
            self.at += 1;
        }
    }

    /// One coordinate. Exponents and a second `.` inside one token are
    /// refused: the tool reads `1e1` as 0 and empties the path on `0.5.5`.
    fn number(&mut self) -> Result<Fixed> {
        self.separators();
        let start = self.at;
        if matches!(self.src.get(self.at), Some(b'+' | b'-')) {
            self.at += 1;
        }
        while self.src.get(self.at).is_some_and(u8::is_ascii_digit) {
            self.at += 1;
        }
        if self.src.get(self.at) == Some(&b'.') {
            self.at += 1;
            while self.src.get(self.at).is_some_and(u8::is_ascii_digit) {
                self.at += 1;
            }
        }
        if self.at == start {
            return Err(self.error("expected a coordinate"));
        }
        if matches!(self.src.get(self.at), Some(b'.' | b'e' | b'E')) {
            return Err(self.error(
                "a coordinate must be followed by a separator: svgtbinencode \
                 reads an exponent as 0 and empties the path on `0.5.5`",
            ));
        }
        let text = String::from_utf8_lossy(&self.src[start..self.at]);
        let value: f32 = text
            .parse()
            .map_err(|_| self.error(&format!("not a coordinate: {text}")))?;
        Ok(Number(f64::from(value)).fixed())
    }

    fn error(&self, what: &str) -> Error {
        Error::Other(format!(
            "path data at offset {}: {what}",
            self.at.min(self.src.len())
        ))
    }
}

/// `previous` mirrored through `point`.
fn reflect(point: (Fixed, Fixed), previous: (Fixed, Fixed)) -> (Fixed, Fixed) {
    (2 * point.0 - previous.0, 2 * point.1 - previous.1)
}
