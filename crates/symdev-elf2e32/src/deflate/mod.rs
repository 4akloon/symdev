//! E32 image "deflate" (`iCompressionType` 0x101F7AFC): Symbian LZ77 + Huffman.
//!
//! Clean-room implementation from `docs/research/e32-deflate-spec.md` (a behavioural
//! spec written by a separate reader of elf2e32_next); section numbers refer to it.
//! The algorithm is one type, `E32Deflate`, whose methods are split across this
//! directory's files by concept: LZ77 parsing (`lz77`), Huffman table construction
//! (`huffman`), bit I/O (`bits`) and canonical decoding (`decoder`).

mod bits;
mod decoder;
mod huffman;
mod lz77;

use bits::BitWriter;
use decoder::Decoder;
use symdev_core::{Error, Result};

/// Compress / decompress an E32 image body.
pub struct E32Deflate;

impl E32Deflate {
    const LL_SYMBOLS: usize = 285;
    const D_SYMBOLS: usize = 44;
    const END: usize = 284;
    const MAX_CODE_LEN: u32 = 27;
    const MIN_MATCH: u32 = 3;
    const MAX_MATCH: u32 = 258;
    const WINDOW: usize = 4096;
    const MTF_SIZE: usize = 28;
    /// Fixed meta-code lengths for symbols 0..28 (§5.1).
    const META_LENGTHS: [u32; 29] = [
        2, 3, 2, 3, 4, 4, 5, 6, 6, 6, 7, 7, 7, 7, 8, 8, 8, 9, 10, 11, 12, 14, 15, 15, 15, 15, 15,
        16, 16,
    ];

    /// Stream for `body` (§3). Errors when the stream would be longer than the body,
    /// as the reference encoder does (§1.6).
    pub fn compress(body: &[u8]) -> Result<Vec<u8>> {
        let tokens = Self::parse(body);
        let mut ll = vec![0u32; Self::LL_SYMBOLS];
        let mut d = vec![0u32; Self::D_SYMBOLS];
        for t in &tokens {
            match *t {
                lz77::Token::Literal(b) => ll[b as usize] += 1,
                lz77::Token::Match { len, dist } => {
                    ll[256 + Self::bucket(len - Self::MIN_MATCH).0 as usize] += 1;
                    d[Self::bucket(dist - 1).0 as usize] += 1;
                }
            }
        }
        ll[Self::END] += 1;
        let ll_lens = Self::code_lengths(&ll)?;
        let d_lens = Self::code_lengths(&d)?;
        let ll_codes = Self::canonical(&ll_lens);
        let d_codes = Self::canonical(&d_lens);

        let mut out = BitWriter::default();
        let lengths: Vec<u32> = ll_lens.iter().chain(&d_lens).copied().collect();
        Self::write_table(&mut out, &lengths);
        for t in &tokens {
            match *t {
                lz77::Token::Literal(b) => out.code(ll_codes[b as usize], ll_lens[b as usize]),
                lz77::Token::Match { len, dist } => {
                    let (lc, le, lx) = Self::bucket(len - Self::MIN_MATCH);
                    let sym = 256 + lc as usize;
                    out.code(ll_codes[sym], ll_lens[sym]);
                    out.bits(lx, le);
                    let (dc, de, dx) = Self::bucket(dist - 1);
                    out.code(d_codes[dc as usize], d_lens[dc as usize]);
                    out.bits(dx, de);
                }
            }
        }
        out.code(ll_codes[Self::END], ll_lens[Self::END]);
        let stream = out.finish();
        if stream.len() > body.len() {
            return Err(Error::Other(format!(
                "E32 deflate stream ({} bytes) longer than body ({} bytes)",
                stream.len(),
                body.len()
            )));
        }
        Ok(stream)
    }

    /// Body of `len` bytes from `stream` (§7).
    pub fn decompress(stream: &[u8], len: usize) -> Result<Vec<u8>> {
        let mut input = bits::BitReader::new(stream);
        let lengths = Self::read_table(&mut input)?;
        let ll = Decoder::new(&lengths[..Self::LL_SYMBOLS])?;
        let d = Decoder::new(&lengths[Self::LL_SYMBOLS..])?;
        let mut out = Vec::with_capacity(len);
        while out.len() < len {
            let s = ll.symbol(&mut input)?;
            if s < 256 {
                out.push(s as u8);
                continue;
            }
            if s == Self::END {
                break;
            }
            let length = Self::MIN_MATCH + Self::unbucket((s - 256) as u32, &mut input)?;
            let dist = 1 + Self::unbucket(d.symbol(&mut input)? as u32, &mut input)? as usize;
            if dist > out.len() {
                return Err(Error::Other(format!(
                    "E32 deflate distance {dist} before start of output"
                )));
            }
            for _ in 0..length {
                out.push(out[out.len() - dist]);
            }
        }
        if out.len() != len {
            return Err(Error::Other(format!(
                "E32 deflate produced {} bytes, header says {len}",
                out.len()
            )));
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests;
