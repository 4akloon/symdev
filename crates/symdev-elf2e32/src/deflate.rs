//! E32 image "deflate" (`iCompressionType` 0x101F7AFC): Symbian LZ77 + Huffman.
//!
//! Clean-room implementation from `docs/research/e32-deflate-spec.md` (a behavioural
//! spec written by a separate reader of elf2e32_next); section numbers refer to it.

use symdev_core::{Error, Result};

/// Compress / decompress an E32 image body.
pub struct E32Deflate;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Token {
    Literal(u8),
    Match { len: u32, dist: u32 },
}

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
                Token::Literal(b) => ll[b as usize] += 1,
                Token::Match { len, dist } => {
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
                Token::Literal(b) => out.code(ll_codes[b as usize], ll_lens[b as usize]),
                Token::Match { len, dist } => {
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
        let mut input = BitReader::new(stream);
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

    /// `(code, extra-bit count, extra value)` for a length (`L - 3`) or distance
    /// (`D - 1`) value (§3.3).
    fn bucket(v: u32) -> (u32, u32, u32) {
        if v < 8 {
            return (v, 0, 0);
        }
        let mut e = 0;
        while (v >> e) >= 8 {
            e += 1;
        }
        (4 * e + (v >> e), e, v & ((1 << e) - 1))
    }

    fn unbucket(code: u32, input: &mut BitReader) -> Result<u32> {
        if code < 8 {
            return Ok(code);
        }
        let e = (code >> 2) - 1;
        Ok(((4 + (code & 3)) << e) + input.bits(e)?)
    }

    /// Code lengths from counts (§4.2).
    fn code_lengths(counts: &[u32]) -> Result<Vec<u32>> {
        let mut lens = vec![0u32; counts.len()];
        let used: Vec<usize> = (0..counts.len()).filter(|&s| counts[s] > 0).collect();
        match used.as_slice() {
            [] => return Ok(lens),
            [only] => {
                lens[*only] = 1;
                return Ok(lens);
            }
            _ => {}
        }
        // Nodes: leaves are symbols; internal nodes keep their two children.
        let mut children: Vec<Option<(usize, usize)>> = Vec::new();
        let mut symbol_of: Vec<Option<usize>> = Vec::new();
        // Work list of (weight, node), non-increasing by weight.
        let mut list: Vec<(u32, usize)> = Vec::new();
        let insert = |list: &mut Vec<(u32, usize)>, item: (u32, usize)| {
            let at = list
                .iter()
                .position(|&(w, _)| w < item.0)
                .unwrap_or(list.len());
            list.insert(at, item);
        };
        for &s in &used {
            children.push(None);
            symbol_of.push(Some(s));
            insert(&mut list, (counts[s], children.len() - 1));
        }
        while list.len() > 1 {
            let (wa, a) = list
                .pop()
                .ok_or_else(|| Error::Other("huffman list".into()))?;
            let (wb, b) = list
                .pop()
                .ok_or_else(|| Error::Other("huffman list".into()))?;
            children.push(Some((b, a)));
            symbol_of.push(None);
            insert(&mut list, (wb.wrapping_add(wa), children.len() - 1));
        }
        let mut stack = vec![(list[0].1, 0u32)];
        while let Some((node, depth)) = stack.pop() {
            match children[node] {
                Some((l, r)) => {
                    stack.push((l, depth + 1));
                    stack.push((r, depth + 1));
                }
                None => {
                    if depth > Self::MAX_CODE_LEN {
                        return Err(Error::Other(format!(
                            "E32 deflate Huffman code deeper than {}",
                            Self::MAX_CODE_LEN
                        )));
                    }
                    if let Some(s) = symbol_of[node] {
                        lens[s] = depth;
                    }
                }
            }
        }
        Ok(lens)
    }

    /// Canonical codes from lengths (§4.3).
    fn canonical(lens: &[u32]) -> Vec<u32> {
        let mut count = [0u32; 28];
        for &l in lens {
            if l > 0 {
                count[l as usize] += 1;
            }
        }
        let mut first = [0u32; 28];
        let mut code = 0u32;
        for n in 1..=Self::MAX_CODE_LEN as usize {
            code <<= 1;
            first[n] = code;
            code += count[n];
        }
        lens.iter()
            .map(|&l| {
                if l == 0 {
                    return 0;
                }
                let c = first[l as usize];
                first[l as usize] += 1;
                c
            })
            .collect()
    }

    /// Code-length table: MTF + bijective base-2 runs + fixed meta code (§5).
    fn write_table(out: &mut BitWriter, lengths: &[u32]) {
        let meta = Self::canonical(&Self::META_LENGTHS);
        let emit = |out: &mut BitWriter, m: usize| out.code(meta[m], Self::META_LENGTHS[m]);
        let mut mtf: Vec<u32> = (0..Self::MTF_SIZE as u32).collect();
        let mut run = 0u32;
        for &x in lengths {
            if x == mtf[0] {
                run += 1;
                continue;
            }
            Self::write_run(out, run, &emit);
            run = 0;
            // Lengths are 0..=27, so x is always somewhere in the list.
            let j = mtf.iter().position(|&v| v == x).unwrap_or(0);
            emit(out, j + 1);
            let v = mtf.remove(j);
            mtf.insert(0, v);
        }
        Self::write_run(out, run, &emit);
    }

    fn write_run(out: &mut BitWriter, run: u32, emit: &impl Fn(&mut BitWriter, usize)) {
        if run == 0 {
            return;
        }
        Self::write_run(out, (run - 1) >> 1, emit);
        emit(out, if run % 2 == 1 { 0 } else { 1 });
    }

    fn read_table(input: &mut BitReader) -> Result<Vec<u32>> {
        let total = Self::LL_SYMBOLS + Self::D_SYMBOLS;
        let meta = Decoder::new(&Self::META_LENGTHS)?;
        let mut mtf: Vec<u32> = (0..Self::MTF_SIZE as u32).collect();
        let mut out = Vec::with_capacity(total);
        let mut run = 0usize;
        while out.len() + run < total {
            let m = meta.symbol(input)?;
            if m <= 1 {
                run = 2 * run + m + 1;
                continue;
            }
            out.extend(std::iter::repeat_n(mtf[0], run));
            run = 0;
            let v = mtf.remove(m - 1);
            mtf.insert(0, v);
            out.push(v);
        }
        out.extend(std::iter::repeat_n(mtf[0], run));
        if out.len() != total {
            return Err(Error::Other(
                "E32 deflate code-length table overruns".into(),
            ));
        }
        Ok(out)
    }

    fn hash(body: &[u8], p: usize) -> usize {
        let x = u32::from(body[p]) | u32::from(body[p + 1]) << 8 | u32::from(body[p + 2]) << 16;
        (x.wrapping_mul(0xAC4B_9B19) >> 24) as usize
    }

    /// LZ77 parse with one-step lazy evaluation (§6.4).
    fn parse(body: &[u8]) -> Vec<Token> {
        let n = body.len();
        let mut tokens = Vec::new();
        if n <= 3 {
            tokens.extend(body.iter().map(|&b| Token::Literal(b)));
            return tokens;
        }
        let mut chains = Chains::new(n);
        let mut pending: Option<(u32, u32)> = None;
        let mut i = 0usize;
        loop {
            let (len, dist) = chains.best(body, i);
            match pending {
                Some((plen, pdist)) if len < plen => {
                    tokens.push(Token::Match {
                        len: plen,
                        dist: pdist,
                    });
                    let skip_end = i + plen as usize - 2;
                    for s in i + 1..=skip_end {
                        if s + 2 < n {
                            chains.insert(body, s);
                        }
                    }
                    i = skip_end;
                    pending = None;
                }
                None if len < Self::MIN_MATCH => tokens.push(Token::Literal(body[i])),
                _ => {
                    if pending.is_some() {
                        tokens.push(Token::Literal(body[i - 1]));
                    }
                    pending = Some((len, dist));
                }
            }
            i += 1;
            if i + 2 >= n {
                break;
            }
        }
        if let Some((plen, pdist)) = pending {
            tokens.push(Token::Match {
                len: plen,
                dist: pdist,
            });
            i = i - 1 + plen as usize;
        }
        tokens.extend(body[i.min(n)..].iter().map(|&b| Token::Literal(b)));
        tokens
    }
}

/// Hash chains over inserted positions (§6.2): per bucket the last position, per
/// position the previous one in the same bucket.
struct Chains {
    head: [Option<usize>; 256],
    prev: Vec<Option<usize>>,
}

impl Chains {
    fn new(n: usize) -> Self {
        Self {
            head: [None; 256],
            prev: vec![None; n],
        }
    }

    fn insert(&mut self, body: &[u8], p: usize) -> Option<usize> {
        let h = E32Deflate::hash(body, p);
        let older = self.head[h];
        self.prev[p] = older;
        self.head[h] = Some(p);
        older
    }

    /// Insert `p`, then the longest match among earlier candidates, nearest first,
    /// stopping at the length limit (§6.3).
    fn best(&mut self, body: &[u8], p: usize) -> (u32, u32) {
        let mut candidate = self.insert(body, p);
        let limit = (E32Deflate::MAX_MATCH as usize).min(body.len() - p);
        let (mut best_len, mut best_dist) = (0usize, 0usize);
        while let Some(q) = candidate {
            if p - q > E32Deflate::WINDOW {
                break;
            }
            let k = (0..limit)
                .take_while(|&i| body[q + i] == body[p + i])
                .count();
            if k == limit {
                return (limit as u32, (p - q) as u32);
            }
            if k > best_len {
                best_len = k;
                best_dist = p - q;
            }
            candidate = self.prev[q];
        }
        (best_len as u32, best_dist as u32)
    }
}

#[derive(Default)]
struct BitWriter {
    bytes: Vec<u8>,
    acc: u8,
    used: u32,
}

impl BitWriter {
    /// `n` low bits of `v`, MSB first (§2.2).
    fn bits(&mut self, v: u32, n: u32) {
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

    fn code(&mut self, code: u32, len: u32) {
        self.bits(code, len);
    }

    /// Pad the last partial byte with 1-bits (§2.3).
    fn finish(mut self) -> Vec<u8> {
        if self.used > 0 {
            let pad = 8 - self.used;
            self.bits((1 << pad) - 1, pad);
        }
        self.bytes
    }
}

struct BitReader<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> BitReader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, pos: 0 }
    }

    fn bit(&mut self) -> Result<u32> {
        let byte = self
            .bytes
            .get(self.pos / 8)
            .ok_or_else(|| Error::Other("E32 deflate stream truncated".into()))?;
        let b = (byte >> (7 - self.pos % 8)) & 1;
        self.pos += 1;
        Ok(u32::from(b))
    }

    fn bits(&mut self, n: u32) -> Result<u32> {
        let mut v = 0;
        for _ in 0..n {
            v = (v << 1) | self.bit()?;
        }
        Ok(v)
    }
}

/// Canonical decoder for one alphabet (§4.3, §7).
struct Decoder {
    /// Per length: first code and the symbols of that length in increasing order.
    by_len: Vec<(u32, Vec<usize>)>,
    single: Option<usize>,
}

impl Decoder {
    fn new(lens: &[u32]) -> Result<Self> {
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

    fn symbol(&self, input: &mut BitReader) -> Result<usize> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn meta_code_matches_spec_table() {
        let codes = E32Deflate::canonical(&E32Deflate::META_LENGTHS);
        let show = |m: usize| {
            let l = E32Deflate::META_LENGTHS[m];
            format!("{:0width$b}", codes[m], width = l as usize)
        };
        assert_eq!(show(0), "00");
        assert_eq!(show(1), "100");
        assert_eq!(show(2), "01");
        assert_eq!(show(14), "11111100");
        assert_eq!(show(21), "11111111111100");
        assert_eq!(show(28), "1111111111111111");
    }

    #[test]
    fn length_and_distance_buckets_match_spec_tables() {
        assert_eq!(E32Deflate::bucket(0), (0, 0, 0));
        assert_eq!(E32Deflate::bucket(8), (8, 1, 0)); // length 11 → sym 264
        assert_eq!(E32Deflate::bucket(255), (27, 5, 31)); // length 258 → sym 283
        assert_eq!(E32Deflate::bucket(4095), (43, 9, 511)); // distance 4096 → sym 43
    }

    #[test]
    fn bijective_runs_match_spec_examples() {
        let cases: [(u32, &[usize]); 7] = [
            (1, &[0]),
            (2, &[1]),
            (3, &[0, 0]),
            (4, &[0, 1]),
            (5, &[1, 0]),
            (6, &[1, 1]),
            (7, &[0, 0, 0]),
        ];
        for (run, want) in cases {
            let got = std::cell::RefCell::new(Vec::new());
            E32Deflate::write_run(&mut BitWriter::default(), run, &|_, m| {
                got.borrow_mut().push(m)
            });
            assert_eq!(got.into_inner(), want, "run {run}");
        }
    }

    #[test]
    fn round_trips_edge_inputs() {
        let mut long = vec![7u8; 5000];
        long.extend((0..3000u32).map(|i| (i * 31 % 251) as u8));
        long.extend_from_slice(&long[100..4200].to_vec());
        for body in [long, b"abcabcabcabcabcabcabcabc".repeat(40)] {
            let stream = E32Deflate::compress(&body).unwrap();
            assert_eq!(E32Deflate::decompress(&stream, body.len()).unwrap(), body);
        }
    }

    #[test]
    fn tiny_body_is_an_error_like_the_reference() {
        assert!(E32Deflate::compress(b"ab").is_err());
    }
}
