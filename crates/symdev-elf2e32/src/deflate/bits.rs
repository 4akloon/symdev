//! Bit-level I/O for the E32 deflate stream (`docs/research/e32-deflate-spec.md` §2).
use symdev_core::{Error, Result};

#[derive(Default)]
pub(super) struct BitWriter {
    bytes: Vec<u8>,
    acc: u8,
    used: u32,
}

impl BitWriter {
    /// `n` low bits of `v`, MSB first (§2.2).
    pub(super) fn bits(&mut self, v: u32, n: u32) {
        for k in (0..n).rev() {
            self.acc = (self.acc << 1) | ((v >> k) & 1) as u8;
            self.used += 1;
            if self.used == 8 {
                self.bytes.push(self.acc);
                self.acc = 0;
                self.used = 0;
            }
        }
    }

    pub(super) fn code(&mut self, code: u32, len: u32) {
        self.bits(code, len);
    }

    /// Pad the last partial byte with 1-bits (§2.3).
    pub(super) fn finish(mut self) -> Vec<u8> {
        if self.used > 0 {
            let pad = 8 - self.used;
            self.bits((1 << pad) - 1, pad);
        }
        self.bytes
    }
}

pub(super) struct BitReader<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> BitReader<'a> {
    pub(super) fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, pos: 0 }
    }

    pub(super) fn bit(&mut self) -> Result<u32> {
        let byte = self
            .bytes
            .get(self.pos / 8)
            .ok_or_else(|| Error::Other("E32 deflate stream truncated".into()))?;
        let b = (byte >> (7 - self.pos % 8)) & 1;
        self.pos += 1;
        Ok(u32::from(b))
    }

    pub(super) fn bits(&mut self, n: u32) -> Result<u32> {
        let mut v = 0;
        for _ in 0..n {
            v = (v << 1) | self.bit()?;
        }
        Ok(v)
    }
}
