//! Compressed Unicode in packed resources: the SCSU (Unicode TR #6) encoder `rcomp`
//! uses, as described in [rcomp-spec.md](../../../docs/research/rcomp-spec.md) §2.

/// What a character can be encoded as. The encoder switches windows only for runs of
/// two or more characters of the same class and quotes a lone one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScsuClass {
    /// Written as itself in single-byte mode.
    Plain,
    /// Expressible by a dynamic window definition byte.
    Window(u8),
    /// Only reachable through a static window (in practice the C0 controls).
    Static(u8),
    /// No window holds it: needs Unicode mode.
    Unencodable,
}

/// SCSU encoder state, reset for every text.
pub struct RscScsu {
    windows: [u32; 8],
    active: usize,
    unicode_mode: bool,
    out: Vec<u8>,
    budget: usize,
}

impl RscScsu {
    const STATIC: [u32; 8] = [
        0x0000, 0x0080, 0x0100, 0x0300, 0x2000, 0x2080, 0x2100, 0x3000,
    ];
    const DYNAMIC: [u32; 8] = [
        0x0080, 0x00c0, 0x0400, 0x0600, 0x0900, 0x3040, 0x30a0, 0xff00,
    ];
    /// Offsets with their own one-byte definition, tried in this order.
    const SPECIAL: [(u32, u8); 7] = [
        (0x00c0, 0xf9),
        (0x0250, 0xfa),
        (0x0370, 0xfb),
        (0x0530, 0xfc),
        (0x3040, 0xfd),
        (0x30a0, 0xfe),
        (0xff60, 0xff),
    ];
    const SQ0: u8 = 0x01;
    const SQU: u8 = 0x0e;
    const SCU: u8 = 0x0f;
    const SC0: u8 = 0x10;
    const SD4: u8 = 0x1c;
    const UC0: u8 = 0xe0;
    const UD4: u8 = 0xec;
    const UQU: u8 = 0xf0;
    const LOOK_AHEAD: usize = 4;

    /// Compressed bytes of one text, or `None` when the encoding reaches `2 * n` bytes:
    /// the caller then stores the text raw (spec §1.1, §2.6).
    pub fn encode(units: &[u16]) -> Option<Vec<u8>> {
        let mut me = Self {
            windows: Self::DYNAMIC,
            active: 0,
            unicode_mode: false,
            out: Vec::with_capacity(units.len()),
            budget: 2 * units.len(),
        };
        let mut ahead: Vec<u32> = Vec::with_capacity(Self::LOOK_AHEAD + 1);
        for &u in units {
            ahead.push(u32::from(u));
            if ahead.len() == Self::LOOK_AHEAD {
                me.step(&mut ahead)?;
            }
        }
        while !ahead.is_empty() {
            me.step(&mut ahead)?;
        }
        (me.out.len() < me.budget).then_some(me.out)
    }

    /// One look-ahead step: flush what is already cheap, then switch for a run of two or
    /// more characters of one class and emit it (spec §2.3).
    fn step(&mut self, ahead: &mut Vec<u32>) -> Option<()> {
        while !self.unicode_mode {
            let Some(&front) = ahead.first() else {
                return Some(());
            };
            let offset = self.windows[self.active];
            if Self::classify(front) == ScsuClass::Plain || (offset..offset + 0x80).contains(&front)
            {
                self.emit(front)?;
                ahead.remove(0);
            } else {
                break;
            }
        }
        let Some(&front) = ahead.first() else {
            return Some(());
        };
        let class = Self::classify(front);
        let run = ahead
            .iter()
            .take_while(|&&c| Self::classify(c) == class)
            .count();
        if run >= 2 {
            self.switch(class)?;
        }
        for _ in 0..run {
            let c = ahead.remove(0);
            self.emit(c)?;
        }
        Some(())
    }

    fn classify(ch: u32) -> ScsuClass {
        if matches!(ch, 0x00 | 0x09 | 0x0a | 0x0d) || (0x20..=0x7f).contains(&ch) {
            return ScsuClass::Plain;
        }
        if let Some(b) = Self::window_byte(ch) {
            return ScsuClass::Window(b);
        }
        match (0..8).find(|&n| (Self::STATIC[n]..Self::STATIC[n] + 0x80).contains(&ch)) {
            Some(n) => ScsuClass::Static(n as u8),
            None => ScsuClass::Unencodable,
        }
    }

    /// The window definition byte that would hold `ch`, if any (spec §2.2).
    fn window_byte(ch: u32) -> Option<u8> {
        if ch < 0x80 || (0x3400..=0xdfff).contains(&ch) {
            return None;
        }
        for (start, byte) in Self::SPECIAL {
            if (start..start + 0x80).contains(&ch) {
                return Some(byte);
            }
        }
        Some(if ch >= 0xe000 {
            (((ch + 0x5400) % 0x1_0000) >> 7) as u8
        } else {
            (ch >> 7) as u8
        })
    }

    fn window_offset(byte: u8) -> u32 {
        match byte {
            0x01..=0x67 => u32::from(byte) * 0x80,
            0x68..=0xa7 => u32::from(byte) * 0x80 + 0xac00,
            _ => Self::SPECIAL
                .iter()
                .find(|(_, b)| *b == byte)
                .map_or(0, |(offset, _)| *offset),
        }
    }

    fn push(&mut self, bytes: &[u8]) -> Option<()> {
        self.out.extend_from_slice(bytes);
        (self.out.len() < self.budget).then_some(())
    }

    fn emit(&mut self, ch: u32) -> Option<()> {
        let (hi, lo) = ((ch >> 8) as u8, ch as u8);
        if self.unicode_mode {
            return if (0xe000..=0xf2ff).contains(&ch) {
                self.push(&[Self::UQU, hi, lo])
            } else {
                self.push(&[hi, lo])
            };
        }
        match Self::classify(ch) {
            ScsuClass::Plain => return self.push(&[lo]),
            ScsuClass::Static(n) => return self.push(&[Self::SQ0 + n, lo]),
            _ => {}
        }
        let offset = self.windows[self.active];
        if (offset..offset + 0x80).contains(&ch) {
            return self.push(&[0x80 + (ch - offset) as u8]);
        }
        match (0..8).find(|&j| (self.windows[j]..self.windows[j] + 0x80).contains(&ch)) {
            Some(j) => self.push(&[Self::SQ0 + j as u8, 0x80 + (ch - self.windows[j]) as u8]),
            None => self.push(&[Self::SQU, hi, lo]),
        }
    }

    /// Mode and window change before a run of `class` (spec §2.5). Window 4 is the only
    /// one the encoder ever redefines.
    fn switch(&mut self, class: ScsuClass) -> Option<()> {
        match class {
            ScsuClass::Unencodable => {
                if !self.unicode_mode {
                    self.unicode_mode = true;
                    return self.push(&[Self::SCU]);
                }
            }
            ScsuClass::Plain => {
                if self.unicode_mode {
                    self.unicode_mode = false;
                    return self.push(&[Self::UC0 + self.active as u8]);
                }
            }
            ScsuClass::Static(_) => {}
            ScsuClass::Window(byte) => {
                let offset = Self::window_offset(byte);
                let known = (0..8).find(|&j| self.windows[j] == offset);
                let unicode = self.unicode_mode;
                self.unicode_mode = false;
                match (known, unicode) {
                    (Some(j), false) => {
                        if j != self.active {
                            self.active = j;
                            return self.push(&[Self::SC0 + j as u8]);
                        }
                    }
                    (Some(j), true) => {
                        self.active = j;
                        return self.push(&[Self::UC0 + j as u8]);
                    }
                    (None, false) => {
                        self.windows[4] = offset;
                        self.active = 4;
                        return self.push(&[Self::SD4, byte]);
                    }
                    (None, true) => {
                        self.windows[4] = offset;
                        self.active = 4;
                        return self.push(&[Self::UD4, byte]);
                    }
                }
            }
        }
        Some(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn enc(text: &str) -> Vec<u8> {
        let units: Vec<u16> = text.encode_utf16().collect();
        RscScsu::encode(&units).expect("compressible")
    }

    /// Experiment 56 and rcomp-spec.md §2.7.
    #[test]
    fn matches_the_recorded_encodings() {
        assert_eq!(enc("Hello"), b"Hello");
        assert_eq!(enc("A\u{431}B"), [0x41, 0x03, 0xb1, 0x42]);
        assert_eq!(enc("Привет"), [0x12, 0x9f, 0xc0, 0xb8, 0xb2, 0xb5, 0xc2]);
        assert_eq!(enc("AA\u{c}BB"), [0x41, 0x41, 0x01, 0x0c, 0x42, 0x42]);
        assert_eq!(enc("\u{e9}\u{431}"), [0xe9, 0x03, 0xb1]);
        assert_eq!(enc("\u{80}\u{91}\u{9e}\u{e9}"), [0x80, 0x91, 0x9e, 0xe9]);
        assert_eq!(enc(&"X".repeat(200)), b"X".repeat(200));
    }

    /// rcomp-spec.md §2.5: paths the recorded examples do not reach.
    #[test]
    fn switches_windows_and_modes_as_the_spec_describes() {
        // A run in a window no dynamic window holds: define window 4 (SD4 + byte).
        assert_eq!(enc("\u{531}\u{532}\u{533}"), [0x1c, 0xfc, 0x81, 0x82, 0x83]);
        // A run in a window that exists already: select it (SC3 for U+0600).
        assert_eq!(enc("\u{621}\u{622}\u{623}"), [0x13, 0xa1, 0xa2, 0xa3]);
        // Characters no window can hold: SCU into Unicode mode, UC0 back out.
        assert_eq!(
            enc("AB\u{4e2d}\u{4e2e}AB"),
            [0x41, 0x42, 0x0f, 0x4e, 0x2d, 0x4e, 0x2e, 0xe0, 0x41, 0x42]
        );
        // A lone windowed character while in Unicode mode is quoted with UQU.
        assert_eq!(
            enc("AB\u{4e2d}\u{4e2e}\u{e000}\u{4e2d}\u{4e2e}ABCD"),
            [
                0x41, 0x42, 0x0f, 0x4e, 0x2d, 0x4e, 0x2e, 0xf0, 0xe0, 0x00, 0x4e, 0x2d, 0x4e, 0x2e,
                0xe0, 0x41, 0x42, 0x43, 0x44
            ]
        );
    }

    #[test]
    fn gives_up_when_the_encoding_would_not_be_shorter() {
        // A lone unencodable character costs SQU and two bytes for one character.
        assert_eq!(RscScsu::encode(&[0x4e2d]), None);
    }
}
