//! The four `bmconv` compressors
//! ([bmconv-spec.md](../../../docs/research/bmconv-spec.md) §7). Each returns
//! `None` when the encoder gives up and the bitmap is stored uncompressed.

/// Run-length encoders, one per stored depth.
pub struct MbmRle;

impl MbmRle {
    /// Bytewise RLE (1, 2, 4 and 8 bpp), compression type 1.
    pub fn bytewise(data: &[u8]) -> Option<Vec<u8>> {
        let len = data.len();
        let guard = if len / 64 < 5 { 4 } else { len / 64 };
        let end = len.saturating_sub(guard);
        let budget = len / 4 + len / 2;
        let mut out = Vec::new();
        let mut at = 0;
        while at < end {
            let byte = data[at];
            let run = data.get(at + 1) == Some(&byte) && data.get(at + 2) == Some(&byte);
            let mut scan;
            if run {
                scan = at + 3;
                while data.get(scan) == Some(&byte) && scan < end {
                    scan += 1;
                }
                Self::runs(&mut out, scan - at, &[byte]);
            } else {
                scan = at;
                while scan < end
                    && !(data.get(scan + 1) == Some(&data[scan])
                        && data.get(scan + 2) == Some(&data[scan]))
                {
                    scan += 1;
                }
                Self::literals(&mut out, &data[at..scan], 1);
            }
            at = scan;
            if out.len() > budget {
                return None;
            }
        }
        let rest = len - at;
        if out.len() + rest > budget {
            return None;
        }
        Self::literals(&mut out, &data[at..], 1);
        Some(out)
    }

    /// 12-bit RLE (type 2): count in the top nibble, always applied.
    pub fn twelve_bit(data: &[u8]) -> Option<Vec<u8>> {
        let words: Vec<u16> = data
            .as_chunks::<2>()
            .0
            .iter()
            .map(|w| u16::from_le_bytes(*w))
            .collect();
        let mut out = Vec::new();
        let mut at = 0;
        while at < words.len() {
            let word = words[at];
            let mut scan = at + 1;
            while words.get(scan) == Some(&word) {
                scan += 1;
            }
            let mut left = scan - at;
            while left > 0 {
                let take = left.min(16);
                let value = ((take as u16 - 1) << 12) | (word & 0x0fff);
                out.extend_from_slice(&value.to_le_bytes());
                left -= take;
            }
            at = scan;
        }
        Some(out)
    }

    /// 16-bit RLE (type 3).
    pub fn sixteen_bit(data: &[u8]) -> Option<Vec<u8>> {
        let words: Vec<[u8; 2]> = data.as_chunks::<2>().0.to_vec();
        let n = words.len();
        let budget = data.len() * 7 / 8;
        let mut out = Vec::new();
        let mut at = 0;
        while at + 1 < n {
            let scan = if words[at + 1] == words[at] {
                let mut scan = at + 1;
                loop {
                    scan += 1;
                    if scan >= n || words[scan] != words[at] {
                        break;
                    }
                }
                Self::runs(&mut out, scan - at, &words[at]);
                scan
            } else {
                let mut scan = at + 1;
                while scan + 1 < n && words[scan + 1] != words[scan] {
                    scan += 1;
                }
                Self::literals(&mut out, &words[at..scan].concat(), 2);
                scan
            };
            at = scan;
            if out.len() > budget {
                return None;
            }
        }
        if at < n {
            Self::literals(&mut out, &words[at..].concat(), 2);
        }
        (out.len() <= budget).then_some(out)
    }

    /// 24-bit RLE (type 4), including the first-iteration look-ahead quirk.
    pub fn twenty_four_bit(data: &[u8]) -> Option<Vec<u8>> {
        let len = data.len();
        let end = len.saturating_sub(3);
        let budget = len * 7 / 8;
        let pixel = |at: usize| &data[at..at + 3];
        let mut out = Vec::new();
        let mut at = 0;
        while at < end {
            let scan = if pixel(at + 3) == pixel(at) {
                let mut scan = at + 3;
                loop {
                    scan += 3;
                    if scan >= len || pixel(scan) != pixel(at) {
                        break;
                    }
                }
                Self::runs(&mut out, (scan - at) / 3, pixel(at));
                scan
            } else {
                let mut scan = at + 6;
                let mut reference = at + 3;
                let mut ahead = at + 9;
                while scan < end {
                    if ahead < len && pixel(ahead) == pixel(reference) {
                        break;
                    }
                    reference = scan + 3;
                    scan += 3;
                    ahead += 3;
                }
                let scan = scan.min(len);
                Self::literals(&mut out, &data[at..scan], 3);
                scan
            };
            at = scan;
            if out.len() > budget {
                return None;
            }
        }
        if at < len {
            Self::literals(&mut out, &data[at..], 3);
        }
        (out.len() <= budget).then_some(out)
    }

    /// A run of `count` items, split into blocks of at most 128.
    fn runs(out: &mut Vec<u8>, count: usize, item: &[u8]) {
        let mut left = count;
        while left > 0 {
            let take = left.min(128);
            out.push((take - 1) as u8);
            out.extend_from_slice(item);
            left -= take;
        }
    }

    /// Literal items (`size` bytes each), split into blocks of at most 128.
    fn literals(out: &mut Vec<u8>, bytes: &[u8], size: usize) {
        let mut rest = bytes;
        while !rest.is_empty() {
            let take = (rest.len() / size).min(128);
            out.push((256 - take) as u8);
            out.extend_from_slice(&rest[..take * size]);
            rest = &rest[take * size..];
        }
    }
}
