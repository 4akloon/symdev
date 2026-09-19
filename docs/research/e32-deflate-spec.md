# E32 image "deflate" compression: behavioural specification

> **Provenance.** This is a clean-room behavioural specification, written on
> 2026-09-19 from reading the elf2e32_next sources (EPL-1.0), for use by an
> engineer writing an independent implementation who has not seen that source.
> It contains no source code, no identifiers taken from the source and no
> copied comments. Every numeric constant and table below is a fact about the
> format or about the reference encoder's observable behaviour.
>
> **Status.** Verified: an independent throwaway decoder and encoder written
> only from this document reproduce the golden files byte for byte (see
> section 9).

Scope: the compression that an E32 image header marks with
`iCompressionType = 0x101F7AFC` (the "deflate" UID). Despite the name it is
**not** RFC 1951 DEFLATE. It is a Symbian-specific LZ77 + Huffman format:
a single block, one pair of Huffman tables, and a table encoding of its own.

Terms used below:

- **body**: the bytes of the image that get compressed.
- **stream**: the compressed bytes that replace the body in the file.
- **token**: a literal byte, a (length, distance) copy, or the end marker.
- **LL alphabet**: the combined literal/length alphabet (285 symbols).
- **D alphabet**: the distance alphabet (44 symbols).

---

## 1. What is compressed, and the file layout

1. The E32 header, and anything that comes after it up to the header field
   `iCodeOffset` (u32 LE at file offset 0x64), is stored **uncompressed**.
   This region includes the export-description bitmap when there is one, so
   the compressed region starts at `iCodeOffset`, **not** at the fixed header
   size. (For the golden `hello.exe`, `iCodeOffset = 0x9C`, which is also the
   header size.)
2. The **body** is every byte from `iCodeOffset` to the end of the
   uncompressed image: code, data, imports, relocations, and so on.
3. The uncompressed length of the body is stored in the J-header field
   `iUncompressedSize` (u32 LE at file offset 0x7C). This is the number of
   bytes the decoder must produce. `iCodeOffset + iUncompressedSize` is a
   multiple of 4 in valid images. Readers check this.
4. The compressed file is `header[0 .. iCodeOffset)` followed immediately by
   the stream. **Nothing follows the stream:** there is no length prefix, no
   trailer and no alignment padding beyond the final partial byte (see 2.3).
   The file size is `iCodeOffset + stream_length`, and it need not be a
   multiple of 4.
5. The header is the same in the compressed and uncompressed files, apart
   from `iCompressionType` at 0x1C (0x101F7AFC compressed, 0 uncompressed)
   and `iHeaderCrc` at 0x14. The CRC covers bytes `[0, iCodeOffset)`. It is
   computed while the compression type field already holds its final value
   (after `iHeaderCrc` is set to 0xC90FDAA2 first). The CRC therefore never
   covers the body or the stream. The CRC algorithm is table-driven reflected
   CRC-32 (polynomial 0xEDB88320), starting value 0, no final XOR. It is not
   zlib's variant, which starts at 0xFFFFFFFF and XORs the result with it.
6. The reference encoder writes the stream into a buffer the size of the
   uncompressed body. If the stream would be **longer** than the body, the
   reference tool fails with an error. It does not fall back to storing the
   body uncompressed. A compatible implementation should report an error too,
   or at least must never emit a stream longer than the body and still claim
   byte identity.

## 2. Bitstream conventions

1. **Bit order.** Bits are packed **most significant bit first**. The first
   bit of the stream is bit 7 of stream byte 0, then bit 6, and so on down
   to bit 0, then bit 7 of byte 1.
2. **Multi-bit fields.** Every multi-bit field is written most significant
   bit first. This applies to Huffman codes and to "extra bits" values. To
   write an `n`-bit value `v` (only its low `n` bits count), write bit
   `n-1` of `v` first and bit 0 last. A decoder reading `n` bits builds the
   value the same way, so the first bit read becomes the most significant
   bit. A zero-bit field writes nothing and reads as 0.
3. **End of stream and padding.** After the last Huffman code (the end
   marker, section 3), a final partially filled byte is completed with
   **1-bits**. If the bit count is already a multiple of 8, nothing is added:
   there is never a whole padding byte. The stream length is
   `ceil(total_bits / 8)` bytes.
4. The stream is one continuous bit sequence. The code-length table and the
   token data follow each other with no byte alignment between them.

## 3. Stream structure and alphabets

The stream is exactly:

1. The **code-length table**: 329 code lengths (285 for the LL alphabet,
   then 44 for the D alphabet), encoded as in section 5.
2. The **token data**: the encoded tokens in order (section 3.3), ending with
   the end-of-stream symbol.
3. Padding with 1-bits up to a byte boundary (2.3).

### 3.1 LL alphabet (285 symbols, numbered 0..284)

| Symbols  | Meaning |
|----------|---------|
| 0..255   | literal byte with that value |
| 256..283 | match-length codes (28 codes; table 3.4) |
| 284      | end of stream (the stream has exactly one, as the final token) |

### 3.2 D alphabet (44 symbols, numbered 0..43)

Distance codes. See table 3.5.

### 3.3 How a token is written

- A **literal** `b` is written as the Huffman code of LL symbol `b`.
- A **match** of length `L` (3..258) at distance `D` (1..4096) is written as
  four fields in this order:
  1. the Huffman code of the LL symbol `256 + c`, where `c` is the length
     code from table 3.4;
  2. the length's extra bits, if there are any;
  3. the Huffman code of the D symbol for the distance (table 3.5);
  4. the distance's extra bits, if there are any.
- **End of stream** is the Huffman code of LL symbol 284.

The two tables share one mapping rule, applied to `v = L - 3` for lengths and
`v = D - 1` for distances:

- If `v < 8`, the code is `v` and there are no extra bits.
- Otherwise, let `e` be the number of right shifts by one needed to bring `v`
  below 8, so that `t = v >> e` is in 4..7. The code is `4*e + t`, and the
  extra bits are the low `e` bits of `v`, written MSB first.

The reverse mapping, for a code `c`: if `c < 8`, then `v = c`. Otherwise
`e = (c >> 2) - 1` and `v = ((4 + (c & 3)) << e) + extra`, where `extra` is
an `e`-bit value read from the stream.

### 3.4 Length codes (LL symbol = 256 + code)

| LL sym | code | extra bits | lengths |
|---|---|---|---|
| 256 | 0 | 0 | 3 |
| 257 | 1 | 0 | 4 |
| 258 | 2 | 0 | 5 |
| 259 | 3 | 0 | 6 |
| 260 | 4 | 0 | 7 |
| 261 | 5 | 0 | 8 |
| 262 | 6 | 0 | 9 |
| 263 | 7 | 0 | 10 |
| 264 | 8 | 1 | 11-12 |
| 265 | 9 | 1 | 13-14 |
| 266 | 10 | 1 | 15-16 |
| 267 | 11 | 1 | 17-18 |
| 268 | 12 | 2 | 19-22 |
| 269 | 13 | 2 | 23-26 |
| 270 | 14 | 2 | 27-30 |
| 271 | 15 | 2 | 31-34 |
| 272 | 16 | 3 | 35-42 |
| 273 | 17 | 3 | 43-50 |
| 274 | 18 | 3 | 51-58 |
| 275 | 19 | 3 | 59-66 |
| 276 | 20 | 4 | 67-82 |
| 277 | 21 | 4 | 83-98 |
| 278 | 22 | 4 | 99-114 |
| 279 | 23 | 4 | 115-130 |
| 280 | 24 | 5 | 131-162 |
| 281 | 25 | 5 | 163-194 |
| 282 | 26 | 5 | 195-226 |
| 283 | 27 | 5 | 227-258 |

### 3.5 Distance codes

| D sym | extra bits | distances | | D sym | extra bits | distances |
|---|---|---|---|---|---|---|
| 0 | 0 | 1 | | 22 | 4 | 97-112 |
| 1 | 0 | 2 | | 23 | 4 | 113-128 |
| 2 | 0 | 3 | | 24 | 5 | 129-160 |
| 3 | 0 | 4 | | 25 | 5 | 161-192 |
| 4 | 0 | 5 | | 26 | 5 | 193-224 |
| 5 | 0 | 6 | | 27 | 5 | 225-256 |
| 6 | 0 | 7 | | 28 | 6 | 257-320 |
| 7 | 0 | 8 | | 29 | 6 | 321-384 |
| 8 | 1 | 9-10 | | 30 | 6 | 385-448 |
| 9 | 1 | 11-12 | | 31 | 6 | 449-512 |
| 10 | 1 | 13-14 | | 32 | 7 | 513-640 |
| 11 | 1 | 15-16 | | 33 | 7 | 641-768 |
| 12 | 2 | 17-20 | | 34 | 7 | 769-896 |
| 13 | 2 | 21-24 | | 35 | 7 | 897-1024 |
| 14 | 2 | 25-28 | | 36 | 8 | 1025-1280 |
| 15 | 2 | 29-32 | | 37 | 8 | 1281-1536 |
| 16 | 3 | 33-40 | | 38 | 8 | 1537-1792 |
| 17 | 3 | 41-48 | | 39 | 8 | 1793-2048 |
| 18 | 3 | 49-56 | | 40 | 9 | 2049-2560 |
| 19 | 3 | 57-64 | | 41 | 9 | 2561-3072 |
| 20 | 4 | 65-80 | | 42 | 9 | 3073-3584 |
| 21 | 4 | 81-96 | | 43 | 9 | 3585-4096 |

Parameters: minimum match 3, maximum match 258, window (maximum distance) 4096.

## 4. Huffman code lengths and canonical codes

### 4.1 Frequencies

The encoder makes **two passes** over the body. Both passes run the same
match finder (section 6) from a fresh state, so both produce the same token
sequence.

- **Pass 1** only counts symbols:
  - each literal adds 1 to its LL symbol;
  - each match adds 1 to its LL length symbol and 1 to its D symbol;
  - the end marker adds 1 to LL symbol 284.

  Extra bits are not counted.
- **Pass 2** writes the tokens using the codes built from those counts.

### 4.2 Code lengths from frequencies

The LL alphabet (285 counts) and the D alphabet (44 counts) are processed
separately and in the same way. The maximum code length is **27**. Lengths
are not limited or rebalanced: if the tree would be deeper than 27, the
reference tool fails with an error. In practice this cannot happen for bodies
of realistic size.

Tree construction:

1. Build a work list that is kept sorted by weight in **non-increasing**
   order (heaviest first, lightest last).
2. Go through the symbols in increasing symbol order. Skip any symbol with
   count 0. Insert each other symbol as a leaf whose weight is its count.
3. **Insertion rule**, used both for leaves and for merged nodes: put the new
   item immediately **after every existing item whose weight is greater than
   or equal to** the new weight. It lands just before the first item that is
   strictly lighter, or at the end if there is none. So among equal weights,
   an item inserted later sits nearer the light end of the list. For leaves
   this means that among equal counts, the lower-numbered symbol is nearer
   the heavy end.
4. Special cases:
   - If no symbol has a non-zero count, every length is 0.
   - If exactly one symbol has a non-zero count, that symbol gets length 1
     and every other symbol gets 0.
5. Otherwise, repeat until one item remains:
   1. Remove the last item (the lightest, call it A).
   2. Remove the new last item (call it B).
   3. Make an internal node with children B and A, whose weight is
      `weight(B) + weight(A)`. Weights are unsigned 32-bit.
   4. Insert the new node using the rule in step 3.
6. The code length of each symbol is the depth of its leaf. The root has
   depth 0, so its children have depth 1. Symbols with count 0 get length 0.

Only the multiset of depths matters: the order of B and A under a node does
not affect any output. The insertion rule does affect which lengths come out,
so it must be followed exactly.

### 4.3 Canonical code assignment

Codes are assigned canonically from the lengths, for each alphabet on its own:

1. For each length `n` from 1 to 27, count `N[n]`, the number of symbols
   with that length.
2. Set `code = 0`. For `n` from 1 to 27: `code = code << 1`, then
   `first[n] = code`, then `code = code + N[n]`.
3. Go through the symbols in increasing order. Each symbol with length
   `n > 0` gets the code `first[n]`, and then `first[n]` goes up by 1.
4. A code of length `n` is written as `n` bits, MSB first.

So shorter codes are numerically smaller than longer ones, taken as bit
strings, and within one length a lower symbol number gets a smaller code.
In the one-symbol case, that symbol's code is the single bit `0`.

When the body contains no matches, all 44 D lengths are 0 and no distance
code is ever written.

## 5. Encoding the code-length table

The 329 lengths (LL 0..284, then D 0..43) are treated as one sequence. It is
transformed in three layers:

1. move-to-front coding;
2. run-length coding of repeats;
3. a fixed prefix code.

The coding state carries straight over from the LL part to the D part.

### 5.1 Meta alphabet (29 symbols, 0..28) and its fixed code

| meta sym | bits | | meta sym | bits |
|---|---|---|---|---|
| 0 | `00` | | 15 | `11111101` |
| 1 | `100` | | 16 | `11111110` |
| 2 | `01` | | 17 | `111111110` |
| 3 | `101` | | 18 | `1111111110` |
| 4 | `1100` | | 19 | `11111111110` |
| 5 | `1101` | | 20 | `111111111110` |
| 6 | `11100` | | 21 | `11111111111100` |
| 7 | `111010` | | 22 | `111111111111010` |
| 8 | `111011` | | 23 | `111111111111011` |
| 9 | `111100` | | 24 | `111111111111100` |
| 10 | `1111010` | | 25 | `111111111111101` |
| 11 | `1111011` | | 26 | `111111111111110` |
| 12 | `1111100` | | 27 | `1111111111111110` |
| 13 | `1111101` | | 28 | `1111111111111111` |
| 14 | `11111100` | | | |

(The bits are written left to right, first bit first. This is simply the
canonical code (4.3) for these lengths: 2,3,2,3,4,4,5,6,6,6,7,7,7,7,8,8,8,
9,10,11,12,14,15,15,15,15,15,16,16.)

Meta symbols 0 and 1 are run digits. Meta symbol `k` for `k` from 2 to 28
means "the value at move-to-front position `k - 1`".

### 5.2 Move-to-front state

- The state is an ordered list of 28 values. Position 0 is always the
  **current value**, meaning the last value emitted.
- It starts as `[0, 1, 2, ..., 27]`, so the current value starts as 0.

### 5.3 Encoding procedure

Keep a run counter `r`, starting at 0. For each length `x` in the 329-entry
sequence:

1. If `x` equals the current value (list position 0), add 1 to `r`. Nothing
   is emitted.
2. Otherwise:
   1. Flush the run: if `r > 0`, emit `r` in the run-length form (5.4), then
      set `r = 0`.
   2. Find the position `j` of `x` in the list. It is 1..27 and never 0.
   3. Emit meta symbol `j + 1`.
   4. Move `x` to the front: take it out of position `j` and put it at
      position 0. Everything that was at positions `0..j-1` moves back one
      place. The old current value is now at position 1.

After the last entry, flush any remaining run the same way.

Note that the leading lengths are compared with the initial current value 0.
A table that starts with zero lengths therefore starts with a run and no MTF
symbol.

### 5.4 Run-length form (bijective base 2)

A run count `r >= 1` is written as a string of digits, most significant digit
first, using two digit meanings:

- meta symbol **0** means digit value 1;
- meta symbol **1** means digit value 2.

The digits are the bijective base-2 representation of `r`. A recursive way to
produce them for `r > 0`:

1. Emit the digits of `(r - 1) >> 1`. If that value is 0, emit nothing.
2. Then emit meta symbol 0 if `r` is odd, or meta symbol 1 if `r` is even.

Examples: 1 is `0`, 2 is `1`, 3 is `0 0`, 4 is `0 1`, 5 is `1 0`,
6 is `1 1`, 7 is `0 0 0`.

### 5.5 Decoding the table

1. Start with the list `[0..27]`, `r = 0` and a count of 0 lengths produced
   so far.
2. While (lengths produced + `r`) is less than 329, read one meta symbol `m`
   using the fixed code:
   - If `m` is 0 or 1, set `r = 2*r + m + 1`.
   - Otherwise:
     1. Output the current value `r` times, then set `r = 0`.
     2. Take the value at list position `m - 1`, move it to the front
        (as in 5.3, step 2.4) and output it once.
3. At the end, output the current value `r` more times.

The result is the 329 lengths.

A conforming decoder also rejects the table unless both parts are valid.
Each part is valid if its codes exactly fill the code space
(the sum of `2^-len` over non-zero lengths equals 1), or it has at most one
code of length 1 and no others, or it is all zeros. No length may exceed 27.

## 6. The encoder's match finder (LZ77 parsing)

This section must be followed exactly for byte-identical output.

### 6.1 Parameters

- Window: candidate distances from 1 to 4096 inclusive.
- Minimum usable match: 3 bytes. Maximum match: 258 bytes.
- Hash: over the three bytes at position `p`:
  1. Form `x = b[p] | b[p+1] << 8 | b[p+2] << 16`.
  2. Compute `h = ((x * 0xAC4B9B19) mod 2^32) >> 24`, which is 0..255.

  There are 256 buckets.

### 6.2 Hash chains

The encoder remembers which positions have been **inserted**. For a position
`p`, the **candidate list** is every earlier inserted position `q` such that:

- `hash(q) == hash(p)`, and
- `1 <= p - q <= 4096`.

The list is ordered from the most recent `q` (smallest distance) to the
oldest. Inserting `p` makes `p` visible to later positions.

The reference implementation keeps, per bucket, the last inserted position.
Per position (modulo 4096) it keeps the gap to the previous inserted position
of the same bucket, capped at 8192. It stops walking a chain as soon as the
distance gone back exceeds 4096. This gives exactly the candidate list above,
including the edge case of a distance of exactly 4096. An implementation may
use any structure that yields the same list in the same order. Candidates are
**not** filtered by comparing actual bytes. A hash collision just gives a
candidate with a short common prefix.

### 6.3 Finding the best match at position `p`

Let `n` be the body length. This step is only called when `p + 2 < n`.

1. Insert `p` into the hash structure **before** its candidate list is used.
   The list is made of positions inserted earlier. `p` itself is not in it.
2. Let `limit = min(258, n - p)`. Set `best_len = 0`.
3. Walk the candidates in order, nearest first. For each candidate `q`:
   1. Count `k`, the number of leading bytes where `b[q+i] == b[p+i]` for
      `i = 0, 1, ...`. Stop at the first mismatch or when `i` reaches
      `limit`. The compared ranges may overlap: `q + i` can reach `p` or go
      past it. Compare the input bytes as they are.
   2. If `k == limit`, the result is (`limit`, `p - q`). Stop searching at
      once.
   3. If `k > best_len`, set `best_len = k` and `best_dist = p - q`. Only a
      strictly longer match replaces the best one, so ties go to the nearer
      candidate.
4. The result is (`best_len`, `best_dist`), or length 0 if there were no
   candidates.

A result with length 3 or more is a usable match. Lengths 0, 1 and 2 (the last
two can come from hash collisions) mean "no match".

### 6.4 Parsing with one-step lazy evaluation

Let `n` be the body length. The parse keeps a **pending** match (a length and
distance, starting at position `i - 1`) or no pending match.

If `n <= 3`, skip the loop: every byte is a literal, followed by the end
marker.

Otherwise, start with `i = 0` and no pending match, and run the loop body
below at least once, repeating while `i + 2 < n`. The first check always
passes, because `n >= 4`.

1. Run 6.3 at position `i`. This inserts `i`. Call the result `M`, with
   length `len(M)`.
2. Then choose exactly one case:
   - **(a)** A pending match `P` exists and `len(M) < len(P)`:
     1. Emit `P` as a match token. It starts at `i - 1`.
     2. Insert each position from `i + 1` to `i + len(P) - 2` inclusive, in
        increasing order, but only those positions `s` with `s + 2 < n`.
        These positions are skipped over, so they are only inserted, and no
        match search runs for them.
     3. Set `i = i + len(P) - 2`, clear the pending match, and go to step 3.
   - **(b)** No pending match and `len(M) < 3`: emit `b[i]` as a literal.
   - **(c)** Otherwise, `len(M) >= 3` and either there is no pending match
     or `len(M) >= len(P)`:
     1. If a pending match exists, emit `b[i - 1]` as a literal. The old
        pending match is discarded.
     2. Make `M` the pending match (starting at `i`).
3. Set `i = i + 1`. In case (a), `i` is now `(start of P) + len(P)`, the
   first byte after the emitted match.
4. Loop again while `i + 2 < n`.

After the loop:

5. If a pending match `P` exists (starting at `i - 1`), emit it and set
   `i = (i - 1) + len(P)`. No hash insertion happens here.
6. Emit every byte from `i` to `n - 1` as a literal.
7. Emit the end-of-stream symbol.

Points to note:

- Ties between the pending match and the new match go to the **new** match:
  `len(M) >= len(P)` replaces the pending match.
- A pending match is only ever emitted in case (a) or step 5. The match found
  at the position right after it is thrown away. No new search starts inside
  or right after an emitted match until the loop reaches the next unsearched
  position.
- Every position the loop reaches in step 1 is inserted exactly once. Skipped
  positions inside a match are inserted in (a).2, subject to `s + 2 < n`.
  Positions inside the match emitted in step 5 are never inserted, but that
  has no effect because no searches follow.
- Distances are always 1 to 4096 and lengths are always 3 to 258.

## 7. Decoder procedure (for verification)

1. Read `iCodeOffset` (0x64) and `iUncompressedSize` (0x7C) from the header.
   Take the stream from `iCodeOffset` to the end of the file.
2. Decode the 329 code lengths (5.5). Split them into LL (the first 285) and
   D (the last 44), and build the canonical codes (4.3) for each. If an
   alphabet has exactly one code of length 1, reading either bit value
   decodes that symbol.
3. Repeat until `iUncompressedSize` bytes have been produced or the end
   symbol has been read:
   1. Decode one LL symbol `s`.
   2. If `s < 256`, output the byte `s`.
   3. If `s == 284`, stop.
   4. Otherwise:
      1. `L = 3 + v`, where `v` comes from length code `s - 256` and its
         extra bits (3.3).
      2. Decode a D symbol `d`, then `D = 1 + v` from code `d` and its extra
         bits.
      3. Copy `L` bytes one at a time from `D` bytes back in the output.
         Copies may overlap.
4. Place the output after the unchanged header bytes `[0, iCodeOffset)`.
   It should be exactly `iUncompressedSize` bytes long. Set
   `iCompressionType` to 0 to get the uncompressed image, and recompute
   `iHeaderCrc` if the header must be valid.
5. Trailing 1-bits after the end symbol are ignored.

## 8. Summary of constants

| item | value |
|---|---|
| compression UID | 0x101F7AFC |
| LL alphabet size | 285 (0..255 literals, 256..283 lengths, 284 end) |
| D alphabet size | 44 |
| code lengths transmitted | 329 (LL, then D) |
| max Huffman code length | 27 (exceeding it is an error, not limited) |
| meta alphabet | 29 symbols, fixed code (5.1) |
| MTF list size | 28, starting as 0..27 |
| min / max match | 3 / 258 |
| window | 4096 |
| hash | 3 bytes as little-endian 24-bit, times 0xAC4B9B19 mod 2^32, top 8 bits |
| bit order | MSB first; final byte padded with 1-bits |

## 9. Worked check against the golden files

Golden files (outside the repo): `/home/genius/src/symdev-experiment-44/`.

- `hello.exe` is 3588 bytes. `iCodeOffset` is 0x9C (156) and
  `iUncompressedSize` is 0x1578 (5496). The stream is bytes 156..3587,
  3432 bytes long.
- `hello_u.exe` is 5652 bytes: 156 header bytes, then the 5496-byte body.

A throwaway Python decoder and encoder were written **only from this
document**. Neither is part of the repo.

- **Decoder check: passed.** Decoding the 3432-byte stream of `hello.exe`
  gives 5496 bytes, identical to bytes 156..5651 of `hello_u.exe`. The end
  symbol comes right after the last byte, and the padding is all 1-bits.
- **Encoder check: passed.** Encoding the 5496-byte body of `hello_u.exe`
  gives a 3432-byte stream, identical to bytes 156..3587 of `hello.exe`.
  Putting `hello_u.exe`'s header in front, with `iCompressionType` set to
  0x101F7AFC and `iHeaderCrc` recomputed, gives `hello.exe` exactly.
