//! LZ77 parsing (`docs/research/e32-deflate-spec.md` §3.3, §6): match tokens, the
//! hash-chain match finder, and the length/distance bucket encoding shared by
//! compression and decompression.
use super::E32Deflate;
use super::bits::BitReader;
use symdev_core::Result;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Token {
    Literal(u8),
    Match { len: u32, dist: u32 },
}

impl E32Deflate {
    /// `(code, extra-bit count, extra value)` for a length (`L - 3`) or distance
    /// (`D - 1`) value (§3.3).
    pub(super) fn bucket(v: u32) -> (u32, u32, u32) {
        if v < 8 {
            return (v, 0, 0);
        }
        let mut e = 0;
        while (v >> e) >= 8 {
            e += 1;
        }
        (4 * e + (v >> e), e, v & ((1 << e) - 1))
    }

    pub(super) fn unbucket(code: u32, input: &mut BitReader) -> Result<u32> {
        if code < 8 {
            return Ok(code);
        }
        let e = (code >> 2) - 1;
        Ok(((4 + (code & 3)) << e) + input.bits(e)?)
    }

    fn hash(body: &[u8], p: usize) -> usize {
        let x = u32::from(body[p]) | u32::from(body[p + 1]) << 8 | u32::from(body[p + 2]) << 16;
        (x.wrapping_mul(0xAC4B_9B19) >> 24) as usize
    }

    /// LZ77 parse with one-step lazy evaluation (§6.4).
    pub(super) fn parse(body: &[u8]) -> Vec<Token> {
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
