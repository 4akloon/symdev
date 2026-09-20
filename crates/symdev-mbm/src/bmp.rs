//! Reading a source BMP the way `bmconv` reads it
//! ([bmconv-spec.md](../../../docs/research/bmconv-spec.md) §3).

use symdev_core::{Error, Result};

/// A decoded source image: pixels top-down, left to right.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BmpImage {
    pub width: u32,
    pub height: u32,
    /// `(r, g, b)` per pixel, `width * height` of them.
    pub pixels: Vec<(u8, u8, u8)>,
    pub x_pels_per_metre: i32,
    pub y_pels_per_metre: i32,
}

impl BmpImage {
    const FILE_HEADER: usize = 14;
    const INFO_HEADER: usize = 40;

    pub fn parse(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < Self::FILE_HEADER + Self::INFO_HEADER || &bytes[..2] != b"BM" {
            return Err(Error::Other("not a BMP".into()));
        }
        let u32_at = |at: usize| {
            u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
        };
        let i32_at = |at: usize| u32_at(at) as i32;
        let file_size = u32_at(2) as usize;
        let width = u32_at(18);
        let height = i32_at(22);
        if height < 0 {
            return Err(Error::Other(
                "TODO: top-down BMP (bmconv crashes on one)".into(),
            ));
        }
        let bit_count = u16::from_le_bytes([bytes[28], bytes[29]]);
        if u32_at(30) != 0 {
            return Err(Error::Other("unknown source compression type".into()));
        }
        let mut colours = u32_at(46) as usize;
        if colours == 0 && !matches!(bit_count, 24 | 32) {
            colours = 1usize << bit_count;
        }
        if colours > 256 {
            return Err(Error::Other(format!(
                "BMP with {colours} palette entries (bmconv rejects more than 256)"
            )));
        }
        let palette_at = Self::FILE_HEADER + Self::INFO_HEADER;
        let data_at = palette_at + 4 * colours;
        let data_len = file_size
            .checked_sub(data_at)
            .ok_or_else(|| Error::Other("BMP size field too small".into()))?;
        if bytes.len() < data_at + data_len {
            return Err(Error::Other("BMP shorter than its size field".into()));
        }
        let palette: Vec<(u8, u8, u8)> = (0..colours)
            .map(|i| {
                let at = palette_at + 4 * i;
                (bytes[at + 2], bytes[at + 1], bytes[at])
            })
            .collect();
        let data = &bytes[data_at..data_at + data_len];
        let stride = Self::source_stride(width, bit_count)?;
        let height = height as u32;
        let mut pixels = Vec::with_capacity((width * height) as usize);
        for y in 0..height {
            // Source rows are bottom-up.
            let row = (height - y - 1) as usize * stride;
            for x in 0..width as usize {
                pixels.push(Self::pixel(data, row, x, bit_count, &palette)?);
            }
        }
        Ok(Self {
            width,
            height,
            pixels,
            x_pels_per_metre: i32_at(38),
            y_pels_per_metre: i32_at(42),
        })
    }

    fn source_stride(width: u32, bit_count: u16) -> Result<usize> {
        let w = width as usize;
        Ok(match bit_count {
            1 => ((w + 31) / 8) & !3,
            4 => ((w + 7) / 2) & !3,
            8 => (w + 3) & !3,
            24 => ((w + 1) * 3) & !3,
            32 => w * 4,
            other => {
                return Err(Error::Other(format!(
                    "TODO: {other}-bit BMP (bmconv reads 1, 4, 8, 24 and 32 usefully)"
                )));
            }
        })
    }

    /// One source pixel as `(r, g, b)`, with the two substitutions bmconv makes
    /// before any depth conversion (spec §3.3).
    fn pixel(
        data: &[u8],
        row: usize,
        x: usize,
        bit_count: u16,
        palette: &[(u8, u8, u8)],
    ) -> Result<(u8, u8, u8)> {
        let byte = |at: usize| data.get(at).copied().unwrap_or(0xff);
        let indexed = |i: usize, grey: u8| match palette.get(i) {
            Some(&c) => c,
            None => (grey, grey, grey),
        };
        let rgb = match bit_count {
            1 => {
                let bit = (byte(row + x / 8) >> (7 - (x % 8))) & 1;
                indexed(usize::from(bit), if bit == 0 { 0 } else { 0xff })
            }
            4 => {
                let b = byte(row + x / 2);
                let nibble = if x.is_multiple_of(2) { b >> 4 } else { b & 0xf };
                indexed(usize::from(nibble), nibble * 0x11)
            }
            8 => {
                let b = byte(row + x);
                indexed(usize::from(b), b)
            }
            24 => (
                byte(row + 3 * x + 2),
                byte(row + 3 * x + 1),
                byte(row + 3 * x),
            ),
            32 => (
                byte(row + 4 * x + 2),
                byte(row + 4 * x + 1),
                byte(row + 4 * x),
            ),
            other => {
                return Err(Error::Other(format!("TODO: {other}-bit BMP pixels")));
            }
        };
        Ok(match rgb {
            (0x80, 0x80, 0x80) => (0x7f, 0x7f, 0x7f),
            (0xc0, 0xc0, 0xc0) => (0xbb, 0xbb, 0xbb),
            other => other,
        })
    }

    /// Twips size from the pixels-per-metre fields (spec §4.2).
    pub fn twips(&self) -> (u32, u32) {
        let axis = |pixels: u32, ppm: i32| {
            if ppm <= 0 {
                0
            } else {
                ((u64::from(pixels) * 1_440_000 / 254) / ppm as u64) as u32
            }
        };
        (
            axis(self.width, self.x_pels_per_metre),
            axis(self.height, self.y_pels_per_metre),
        )
    }
}
