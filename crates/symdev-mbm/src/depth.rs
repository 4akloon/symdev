//! Target depths: stride, pixel conversion and the two built-in palettes
//! ([bmconv-spec.md](../../../docs/research/bmconv-spec.md) §4).

use symdev_core::{Error, Result};

use crate::bmp::BmpImage;

/// A `bmconv` depth option (`/8`, `/c12`, …).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MbmDepth {
    Grey1,
    Grey2,
    Grey4,
    Grey8,
    Colour4,
    Colour8,
    Colour12,
    Colour16,
    Colour24,
}

impl MbmDepth {
    /// The six levels of the 6×6×6 cube the 256-colour palette is built from.
    const LEVELS: [u8; 6] = [0x00, 0x33, 0x66, 0x99, 0xcc, 0xff];
    /// Indices 108…147 of the 256-colour palette, as `0x00BBGGRR` words.
    const EXTRA: [u32; 40] = [
        0x111111, 0x222222, 0x444444, 0x555555, 0x777777, 0x000011, 0x000022, 0x000044, 0x000055,
        0x000077, 0x001100, 0x002200, 0x004400, 0x005500, 0x007700, 0x110000, 0x220000, 0x440000,
        0x550000, 0x770000, 0x880000, 0xaa0000, 0xbb0000, 0xdd0000, 0xee0000, 0x008800, 0x00aa00,
        0x00bb00, 0x00dd00, 0x00ee00, 0x000088, 0x0000aa, 0x0000bb, 0x0000dd, 0x0000ee, 0x888888,
        0xaaaaaa, 0xbbbbbb, 0xdddddd, 0xeeeeee,
    ];
    /// The 16-colour palette, as `0x00BBGGRR` words.
    const PALETTE16: [u32; 16] = [
        0x000000, 0x555555, 0x000080, 0x008080, 0x008000, 0x0000ff, 0x00ffff, 0x00ff00, 0xff00ff,
        0xff0000, 0xffff00, 0x800080, 0x800000, 0x808000, 0xaaaaaa, 0xffffff,
    ];

    pub fn from_option(option: &str) -> Result<Self> {
        Ok(match option {
            "1" => Self::Grey1,
            "2" => Self::Grey2,
            "4" => Self::Grey4,
            "8" => Self::Grey8,
            "c4" => Self::Colour4,
            "c8" => Self::Colour8,
            "c12" => Self::Colour12,
            "c16" => Self::Colour16,
            "c24" => Self::Colour24,
            other => return Err(Error::Other(format!("unknown bitmap depth /{other}"))),
        })
    }

    pub fn bits(self) -> u32 {
        match self {
            Self::Grey1 => 1,
            Self::Grey2 => 2,
            Self::Grey4 | Self::Colour4 => 4,
            Self::Grey8 | Self::Colour8 => 8,
            Self::Colour12 => 12,
            Self::Colour16 => 16,
            Self::Colour24 => 24,
        }
    }

    pub fn is_colour(self) -> bool {
        matches!(
            self,
            Self::Colour4 | Self::Colour8 | Self::Colour12 | Self::Colour16 | Self::Colour24
        )
    }

    /// Bytes per stored row (spec §4.6): the usual 4-byte rounding, except that
    /// 12 bpp takes two whole bytes per pixel and 24 bpp rounds to four pixels.
    pub fn stride(self, width: u32) -> usize {
        let w = width as usize;
        match self.bits() {
            1 => w.div_ceil(32) * 4,
            2 => w.div_ceil(16) * 4,
            4 => w.div_ceil(8) * 4,
            8 => w.div_ceil(4) * 4,
            12 | 16 => w.div_ceil(2) * 4,
            _ => (w * 3).div_ceil(12) * 12,
        }
    }

    /// The whole pixel buffer, rows top-down, padding bytes and bits left at 1.
    pub fn encode(self, image: &BmpImage) -> Vec<u8> {
        let stride = self.stride(image.width);
        let mut out = vec![0xffu8; stride * image.height as usize];
        for y in 0..image.height as usize {
            let row = y * stride;
            for x in 0..image.width as usize {
                let (r, g, b) = image.pixels[y * image.width as usize + x];
                match self.bits() {
                    bits @ (1 | 2 | 4) => {
                        let value = self.value(r, g, b) & ((1 << bits) - 1);
                        let per_byte = 8 / bits as usize;
                        let at = row + x / per_byte;
                        let shift = (x % per_byte) * bits as usize;
                        out[at] &= !(((1u32 << bits) - 1) << shift) as u8;
                        out[at] |= (value << shift) as u8;
                    }
                    8 => out[row + x] = self.value(r, g, b) as u8,
                    12 | 16 => {
                        let word = self.value(r, g, b) as u16;
                        out[row + 2 * x..row + 2 * x + 2].copy_from_slice(&word.to_le_bytes());
                    }
                    _ => {
                        out[row + 3 * x] = b;
                        out[row + 3 * x + 1] = g;
                        out[row + 3 * x + 2] = r;
                    }
                }
            }
        }
        out
    }

    /// The stored value of one pixel (24 bpp writes its three bytes directly).
    fn value(self, r: u8, g: u8, b: u8) -> u32 {
        let grey = (2 * u32::from(r) + 5 * u32::from(g) + u32::from(b)) / 8;
        match self {
            Self::Grey1 => grey / 128,
            Self::Grey2 => grey / 64,
            Self::Grey4 => grey / 16,
            Self::Grey8 => grey,
            // The tool's 512-entry table is the nearest palette entry for the top
            // three bits of each channel (spec §4.4).
            Self::Colour4 => u32::from(Self::nearest(
                &Self::PALETTE16,
                (u32::from(r) / 32 * 255 / 7) as u8,
                (u32::from(g) / 32 * 255 / 7) as u8,
                (u32::from(b) / 32 * 255 / 7) as u8,
            )),
            Self::Colour8 => u32::from(Self::nearest(
                &Self::palette256(),
                (r >> 4) * 0x11,
                (g >> 4) * 0x11,
                (b >> 4) * 0x11,
            )),
            Self::Colour12 => {
                (u32::from(r) / 16) * 256 + (u32::from(g) / 16) * 16 + u32::from(b) / 16
            }
            Self::Colour16 => {
                (u32::from(r & 0xf8) * 256) + (u32::from(g & 0xfc) * 8) + u32::from(b) / 8
            }
            Self::Colour24 => 0,
        }
    }

    /// Indices 0…107 and 148…255 are the colour cube, 108…147 the ramps.
    fn palette256() -> [u32; 256] {
        let cube = |k: usize| {
            let level = |i: usize| u32::from(Self::LEVELS[i]);
            level(k % 6) | (level((k / 6) % 6) << 8) | (level(k / 36) << 16)
        };
        let mut out = [0u32; 256];
        for (i, slot) in out.iter_mut().enumerate() {
            *slot = match i {
                0..=107 => cube(i),
                108..=147 => Self::EXTRA[i - 108],
                _ => cube(i - 40),
            };
        }
        out
    }

    /// Nearest palette entry by city-block distance, first index on a tie
    /// (spec §4.4: this reproduces both of the tool's lookup tables).
    fn nearest(palette: &[u32], r: u8, g: u8, b: u8) -> u8 {
        let mut best = (u32::MAX, 0usize);
        for (i, &word) in palette.iter().enumerate() {
            let (pr, pg, pb) = (word & 0xff, (word >> 8) & 0xff, (word >> 16) & 0xff);
            let distance =
                pr.abs_diff(u32::from(r)) + pg.abs_diff(u32::from(g)) + pb.abs_diff(u32::from(b));
            if distance < best.0 {
                best = (distance, i);
            }
        }
        best.1 as u8
    }
}
