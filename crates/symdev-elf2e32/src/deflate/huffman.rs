//! Huffman code-length table construction and its bijective-run encoding
//! (`docs/research/e32-deflate-spec.md` §4, §5).
use super::E32Deflate;
use super::bits::{BitReader, BitWriter};
use super::decoder::Decoder;
use symdev_core::{Error, Result};

impl E32Deflate {
    /// Code lengths from counts (§4.2).
    pub(super) fn code_lengths(counts: &[u32]) -> Result<Vec<u32>> {
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
    pub(super) fn canonical(lens: &[u32]) -> Vec<u32> {
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
    pub(super) fn write_table(out: &mut BitWriter, lengths: &[u32]) {
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

    pub(super) fn write_run(out: &mut BitWriter, run: u32, emit: &impl Fn(&mut BitWriter, usize)) {
        if run == 0 {
            return;
        }
        Self::write_run(out, (run - 1) >> 1, emit);
        emit(out, if run % 2 == 1 { 0 } else { 1 });
    }

    pub(super) fn read_table(input: &mut BitReader) -> Result<Vec<u32>> {
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
}
