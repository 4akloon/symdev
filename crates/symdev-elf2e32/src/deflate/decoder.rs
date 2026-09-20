//! `Decoder`: canonical Huffman decoding for one alphabet (§4.3, §7).
use super::E32Deflate;
use super::bits::BitReader;
use symdev_core::{Error, Result};

/// Canonical decoder for one alphabet (§4.3, §7).
pub(super) struct Decoder {
    /// Per length: first code and the symbols of that length in increasing order.
    by_len: Vec<(u32, Vec<usize>)>,
    single: Option<usize>,
}

impl Decoder {
    pub(super) fn new(lens: &[u32]) -> Result<Self> {
        if lens.iter().any(|&l| l > E32Deflate::MAX_CODE_LEN) {
            return Err(Error::Other("E32 deflate code length over 27".into()));
        }
        let used: Vec<usize> = (0..lens.len()).filter(|&s| lens[s] > 0).collect();
        let single = match used.as_slice() {
            [only] if lens[*only] == 1 => Some(*only),
            _ => None,
        };
        let mut by_len = vec![(0u32, Vec::new()); E32Deflate::MAX_CODE_LEN as usize + 1];
        for &s in &used {
            by_len[lens[s] as usize].1.push(s);
        }
        let mut code = 0u32;
        for entry in by_len.iter_mut().skip(1) {
            code <<= 1;
            entry.0 = code;
            code += entry.1.len() as u32;
        }
        Ok(Self { by_len, single })
    }

    pub(super) fn symbol(&self, input: &mut BitReader) -> Result<usize> {
        if let Some(s) = self.single {
            input.bit()?;
            return Ok(s);
        }
        let mut code = 0u32;
        for (first, syms) in self.by_len.iter().skip(1) {
            code = (code << 1) | input.bit()?;
            if code >= *first && ((code - first) as usize) < syms.len() {
                return Ok(syms[(code - first) as usize]);
            }
        }
        Err(Error::Other("E32 deflate invalid Huffman code".into()))
    }
}
