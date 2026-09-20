# `rcomp` 8.1 (build 004) — behavioural specification

## Provenance

Written 2026-09-20 by a separate agent (the "spec writer") from disassembly of
`epoc32/tools/rcomp.exe` of the S60 3rd FP2 SDK ("Resource compiler version 8.1 (Build 004)",
PE32 i386, 188416 bytes) plus black-box runs of that binary under Wine.
The repository owner explicitly approved disassembling this binary for this purpose (2026-09-19).
The implementer of the Rust reimplementation did not see the disassembly, decompiled code or any
scratch work; this document is the only channel between the two roles. It describes observable
behaviour only.

Everything marked "verified" below was reproduced by running the real binary and comparing bytes.
Two independent cross-checks were run against the model this document describes:

* 160 randomly generated resources (mixed `BYTE`/`WORD`/`LONG`/`BUF`/`LTEXT`/`LTEXT8`/`BUF8`
  members, random text including Cyrillic, Greek, CJK, halfwidth forms, control characters)
  — the resource bytes, the largest-uncompressed value and the packed bit matched exactly in
  160/160 cases.
* All 1296 compressed runs found in the 143-file golden corpus were decoded with a standard
  SCSU decoder and re-encoded with the encoder described in section 2 — 1296/1296 re-encoded
  byte-identically.

Anything that could not be determined is marked **unknown**.

All run invocations used the Unicode mode switch (`-u`), which is what the golden corpus uses.
Non-Unicode output is out of scope for this document.

---

## 0. Terminology

* **resource image** — the bytes of one resource as they appear in the `.rsc` file.
* **uncompressed image** — the bytes the same resource would have if no text were compressed:
  every 16-bit text stored as raw UTF-16LE, preceded by its alignment pad if needed.
  This is the image whose size goes into the `u16` "largest uncompressed resource" header field
  and it is also the coordinate system in which alignment is decided.
* **text** — a run of 16-bit characters produced by one `BUF`/`LTEXT`/`TEXT` (i.e. `BUF16`,
  `LTEXT16`, `TEXT16`) member. The `LTEXT` length byte and the `TEXT` NUL terminator are
  discussed per type in section 3. 8-bit text (`BUF8`, `LTEXT8`, `TEXT8`) is never compressed
  and never aligned.
* **run** — in a packed resource image, a maximal stretch of bytes that is either compressed or
  raw. Runs alternate, and the first run is always a compressed one (possibly of length 0).

---

## 1. When text is compressed

This is a two-level decision. The first level is per text and is cheap; the second level is a
whole-resource fixed-point iteration that can undo the first level's decision.

### 1.1 Level one — per text, at the moment the text is written

When a 16-bit text of *n* characters is added to the resource image:

1. It is compressed with the encoder of section 2, with a hard output budget of `2 * n` bytes
   (the encoder stops as soon as it has produced `2 * n` bytes).
2. Let `c` be the number of bytes produced.
   * If `c < 2 * n`: the **compressed** bytes go into the image, and the text is recorded as a
     compressible text together with a copy of its raw UTF-16LE bytes.
   * Otherwise (`c >= 2 * n`, including the budget-exhausted case): the **raw UTF-16LE** bytes
     go into the image and the text is recorded only as an *alignment point* (section 3.6).
     No compression is possible for this text any more.
3. A text of zero characters takes neither branch: nothing is written and no record is made.
   (`0 >= 0`, and the raw branch is skipped because there are no bytes.)

Note that the alignment pad and the run headers play no part in this first comparison.

### 1.2 The leading empty compressed text

The very first time a *compressible* text is recorded in a resource, and only if something
already precedes it in the resource image — that is, if the image is already non-empty, or if a
length-prefix placeholder (section 5.5) is already pending — an extra **empty compressible text**
is recorded at image offset 0, with zero raw bytes and zero compressed bytes.

This is what produces the leading `00` (empty compressed run) seen in most packed resources.
If the first text starts at offset 0 of the resource with nothing before it, no such record is
made and the resource image starts directly with the first real compressed run.

Verified: `STRUCT AA { BUF a; }` with `a = "Hello"` gives resource bytes
`05 48 65 6c 6c 6f` (one compressed run of 5, no leading empty run), whereas
`STRUCT AA { LTEXT a; }` with the same text gives `00 01 05 05 48 65 6c 6c 6f`
(empty compressed run, raw run holding the length byte, then the compressed run).

### 1.3 Level two — the whole-resource profitability pass

The resource is then laid out in three passes over its recorded text/alignment points. Pass 0
measures, pass 1 decides, pass 2 emits. If pass 1 decides to undo a text, that text is converted
back to raw in place and **the whole three-pass sequence restarts from scratch**. The sequence
therefore runs until pass 1 makes no change.

Quantities tracked while walking the resource in image order:

* **B** — the offset in the *uncompressed image*. It starts at the number of plain bytes before
  the first recorded point and is advanced by: every stretch of plain bytes; the raw size of each
  compressible text (see below); one byte for each alignment pad actually inserted; and the size
  of each length prefix.
* **A** — the number of *raw* (uncompressed-image) bytes accumulated since the end of the
  previous compressible text. It is reset to 0 at the start of every compressible text and is
  advanced by exactly the same things as B.

For a compressible text, its **raw size** is `pad + 2 * n`, where `pad` is 1 if B is odd at the
point the text starts and 0 if B is even, and `2 * n` is the length of its stored raw UTF-16LE
copy. Its **compressed size** `c` is the distance between its start and end positions in the
current resource image.

Pass 0 computes, for every compressible text, the value **T** = the number of raw bytes that
follow its compressed run before the next compressible text (or before the end of the resource,
for the last one). This is exactly the A value observed at the next compressible text, or the
final A at the end of the resource. Pass 0 also records the *uncompressed image size* (the final
B) and remembers which compressible text is the **last** one.

Pass 1 walks the same list. For each compressible text, in order:

* If it is the **first** compressible text processed in this pass **and** it is not the last one,
  it is kept unconditionally, with no test. (The format always begins with a compressed run, so
  that first run header is free.)
* Otherwise the cost test runs:

  ```
  cost = c + header_size(c)
  if not (this is the last compressible text and T == 0):
      cost = cost + header_size(T)
  keep compressed  <=>  cost < raw_size
  ```

  where `header_size(x)` is 1 when `0 <= x <= 0x7F` and 2 otherwise (section 1.5), `c` is the
  compressed size, `T` is the trailing-raw-byte count from pass 0, and `raw_size = pad + 2 * n`.
  Note the comparison is strict: `cost == raw_size` means **do not compress**.

  The `header_size(T)` term accounts for the raw-run header that has to be written after this
  compressed run in order to get back to raw bytes. It is omitted only when this compressed run
  is the last one in the resource and nothing raw follows it, so no further header is needed.

* If the test fails, the text is reverted: its compressed bytes in the image are replaced by its
  raw UTF-16LE bytes (all later recorded positions shift accordingly), the text stops being a
  compressible text and becomes a plain alignment point instead (but only if its compressed size
  was non-zero — reverting the empty leading text leaves no alignment point). Then the whole
  three-pass sequence restarts.
* If the reverted text was simultaneously the first *and* the last compressible text, the
  resource additionally loses its "contains compressible text" state.

Pass 2 emits the packed image (section 1.5).

### 1.4 When the resource ends up not packed

The resource is marked **packed** in the `.rsc` header bitmap if and only if pass 0 of the final
iteration saw at least one compressible text. If every text was reverted, no compressible texts
remain, the packed bit is **clear**, and the emitted bytes are the plain uncompressed image:
16-bit texts as raw UTF-16LE, each preceded by an `0xAB` pad byte when it would otherwise start
at an odd offset.

So: **the `0xAB` pad is not specific to packed resources.** It is the pad byte used by the
alignment points, in both packed and unpacked resources. In a packed resource it is written only
for texts that are stored raw (reverted texts, or texts that lost the level-one test); the pad in
front of a *compressed* run is dropped, which is exactly why the uncompressed image is bigger.

Verified examples (all with `-u`):

| source | resource bytes | packed | largest uncompressed |
|---|---|---|---|
| `STRUCT AA { BYTE b; BUF a; } RESOURCE AA r1 { b=0x12; a = <0x0431>; }` | `12 ab 31 04` | no | 4 |
| `STRUCT AA { LTEXT a; } RESOURCE AA r1 { a = <0x0431>; }` | `01 ab 31 04` | no | 4 |
| `STRUCT AA { LTEXT a; LTEXT b; } RESOURCE AA r1 { a = <0x0431>; b = <0x0432>; }` | `01 ab 31 04 01 ab 32 04` | no | 8 |
| `STRUCT AA { BUF a; WORD b; } RESOURCE AA r1 { a = "Hi"; b=7; }` | `48 00 69 00 07 00` | no | 6 |

The last one is instructive: the text is at offset 0 so there is no leading empty run and the
text is both first and last. `c = 2`, `T = 2` (the trailing `WORD`), `raw_size = 0 + 4`, so
`cost = 2 + 1 + 1 = 4`, which is not `< 4` — reverted, and because it was first and last the
resource loses its packed state entirely.

### 1.5 The packed image format

Runs alternate, starting with a compressed run. Each run is preceded by its length:

* length `0x0000 .. 0x007F` — one byte, the length itself;
* length `0x0080 .. 0x7FFF` — two bytes: `0x80 | (length >> 8)` then `length & 0xFF`.
  Note that for `0x0080 .. 0x00FF` the first byte is `0x80` (the high part is zero) — the
  two-byte form is chosen by magnitude, not by whether the high byte is non-zero.

Verified: a 200-character ASCII `BUF` yields a compressed run of 200 bytes introduced by
`80 c8`.

Pass 2 emits, in image order:

1. the plain bytes before the first recorded point;
2. for each compressible text: the header for its compressed size, its compressed bytes, and then
   — unless it is the last compressible text and `T == 0` — the header for `T`, the number of raw
   bytes that follow;
3. for each alignment point: a single `0xAB` byte if B is odd there, nothing otherwise;
4. all plain byte stretches in between, verbatim.

Two adjacent compressible texts with nothing between them therefore produce a raw run of length
0 between the two compressed runs.

Verified: `STRUCT AA { BUF a; BUF b; } RESOURCE AA r1 { a="Hello"; b="World"; }` gives
`05 48 65 6c 6c 6f 00 05 57 6f 72 6c 64` — compressed(5) "Hello", raw(0), compressed(5) "World".

### 1.6 Worked example — the nine one-character texts of `golden/024`

Source shape (reproduced exactly with the real binary):

```
STRUCT AA { LTEXT a[]; }
RESOURCE AA r1 { a = {"2","2","2","2","2","2","2","2","2"}; }
```

Output resource bytes (identical to `golden/024_imopenapiexample.rsc` resource 8 at file
offset 457), with `largest uncompressed = 38` and the packed bit set:

```
00 23 09 00 01 ab 32 00 01 ab 32 00 01 ab 32 00 01 ab 32 00
01 ab 32 00 01 ab 32 00 01 ab 32 00 01 ab 32 00 01 01 32
```

Why the first eight stay raw and only the ninth is compressed:

* Level one compresses every `"2"`: `c = 1 < 2 = 2 * n`, so initially all nine are compressible
  texts, plus the empty leading one.
* Pass 1, first iteration: the leading empty text is first but not last, so it is kept. The next
  text has `c = 1`, `T = 1` (the following `LTEXT` length byte), `raw_size = 1 + 2 = 3`. Cost is
  `1 + 1 + 1 = 3`, not `< 3` — reverted. Restart.
* The same arithmetic applies to each following text in turn; eight of them revert, one per
  iteration.
* For the ninth (now the last compressible text) `T = 0`, so the trailing header term is dropped:
  cost is `1 + 1 = 2 < 3` — kept compressed.
* Final layout: empty compressed run (`00`), then a raw run of 35 bytes (`23`) covering the array
  count `09 00`, the eight raw texts each with its `01` length byte and `0xAB` pad, and the ninth
  text's `01` length byte, then a compressed run of 1 byte (`01 32`).
* Uncompressed image: `2 + 9 * (1 + 1 + 2) = 38`.

---

## 2. The text encoder (SCSU, UTS #6)

The encoder is a standard SCSU encoder with a specific, small set of heuristics. It is
**reset for every text** — window table, active window and mode all start from scratch at the
beginning of each text, and nothing carries over between texts or resources.

### 2.1 Tables

Static windows (used only via single quotes, see 2.4):

| index | offset |
|---|---|
| 0 | `0x0000` |
| 1 | `0x0080` |
| 2 | `0x0100` |
| 3 | `0x0300` |
| 4 | `0x2000` |
| 5 | `0x2080` |
| 6 | `0x2100` |
| 7 | `0x3000` |

Dynamic window initial offsets:

| index | offset |
|---|---|
| 0 | `0x0080` |
| 1 | `0x00C0` |
| 2 | `0x0400` |
| 3 | `0x0600` |
| 4 | `0x0900` |
| 5 | `0x3040` |
| 6 | `0x30A0` |
| 7 | `0xFF00` |

Window-definition byte to offset (standard UTS #6 mapping):

| byte `b` | offset |
|---|---|
| `0x01 .. 0x67` | `b * 0x80` |
| `0x68 .. 0xA7` | `b * 0x80 + 0xAC00` |
| `0xF9` | `0x00C0` |
| `0xFA` | `0x0250` |
| `0xFB` | `0x0370` |
| `0xFC` | `0x0530` |
| `0xFD` | `0x3040` |
| `0xFE` | `0x30A0` |
| `0xFF` | `0xFF60` |
| anything else | 0 (never used) |

Initial state: single-byte mode; active dynamic window index 0, active offset `0x0080`.

### 2.2 Character classification

Every character is classified once, before any encoding decision:

1. **Directly representable** if it is `U+0000`, `U+0009`, `U+000A`, `U+000D`, or in
   `U+0020 .. U+007F`. Call this class *plain*. Note `U+007F` is included and the other C0
   controls are not.
2. Otherwise, the encoder tries to find a *dynamic window definition byte* for it:
   * characters below `U+0080` have none;
   * characters in `U+3400 .. U+DFFF` have none (they are not expressible as a window);
   * a character in `[s, s + 0x80)` for one of the special offsets `0x00C0`, `0x0250`, `0x0370`,
     `0x0530`, `0x3040`, `0x30A0`, `0xFF60` gets the corresponding byte `0xF9 .. 0xFF`
     (first match wins, in that order);
   * a character at or above `U+E000` gets `((ch + 0x5400) mod 0x10000) >> 7`;
   * anything else gets `ch >> 7`.

   If a byte was found, the class is that byte value (`0x01 .. 0xFF`).
3. Otherwise the encoder looks for a *static* window containing it. In practice only the C0
   control characters that are not plain (`U+0001 .. U+0008`, `U+000B`, `U+000C`,
   `U+000E .. U+001F`) reach this step and they all land in static window 0. Call this class
   *static n*.
4. Otherwise (no window at all — in practice only `U+3400 .. U+DFFF`, i.e. CJK extension A, the
   main CJK block and the surrogate range) the class is *unencodable*.

### 2.3 Look-ahead

Characters are pushed into a **4-entry ring buffer**. Whenever the buffer becomes full, and once
more repeatedly at the end of the text until it is empty, the following step runs:

1. **Flush the cheap prefix.** Only while in single-byte mode, repeatedly take the front
   character and emit it immediately if either its class is *plain*, or its value lies in
   `[active offset, active offset + 0x80)`. Stop at the first character that is neither.
2. **Measure the run.** Let *k* be the class of the (new) front character. Count how many
   characters from the front, consecutively, have exactly that same class — at most as many as
   the buffer holds.
3. **Switch, if worth it.** If that count is 2 or more, perform a mode/window change for class
   *k* (section 2.5) before emitting.
4. **Emit.** Emit that many characters from the front (section 2.4).
5. Move any pending output bytes to the output buffer, respecting the output budget.

So the encoder never looks more than 4 characters ahead, and a lone character of some class is
always quoted rather than switched to.

### 2.4 Emitting one character

**In single-byte mode:**

| case | bytes |
|---|---|
| class *plain* | `ch & 0xFF` (the character is `<= 0x7F`) |
| class *static n* | `0x01 + n` (SQ*n*), then `ch & 0xFF` |
| `ch` in the active dynamic window | `0x80 + (ch - active offset)` |
| `ch` in some other dynamic window *j* (lowest *j* first) | `0x01 + j` (SQ*j*), then `0x80 + (ch - window[j])` |
| otherwise | `0x0E` (SQU), then `ch >> 8`, then `ch & 0xFF` |

**In Unicode mode:**

| case | bytes |
|---|---|
| `0xE000 <= ch <= 0xF2FF` | `0xF0` (UQU), then `ch >> 8`, then `ch & 0xFF` |
| otherwise | `ch >> 8`, then `ch & 0xFF` |

### 2.5 Switching for a run of 2 or more characters of the same class

| class of the run | in single-byte mode | in Unicode mode |
|---|---|---|
| *unencodable* | emit `0x0F` (SCU), enter Unicode mode | nothing (already there) |
| *plain* | nothing | emit `0xE0 + active index` (UC*n*), leave Unicode mode |
| *static n* | nothing (static windows are never switched to) | nothing |
| window byte `b`, and some dynamic window *j* already has that offset | if `j` differs from the active index, emit `0x10 + j` (SC*j*); if it is already active, nothing. Active index becomes *j* | emit `0xE0 + j` (UC*j*), leave Unicode mode, active index becomes *j* |
| window byte `b`, no window has that offset | emit `0x1C` (SD4), then `b`; window 4 is redefined to that offset, active index becomes 4 | emit `0xEC` (UD4), then `b`; window 4 is redefined, leave Unicode mode, active index becomes 4 |

Two consequences worth stating explicitly:

* **Window 4 is the only dynamic window the encoder ever redefines.** `SD0..SD3`, `SD5..SD7`,
  `UD0..UD3`, `UD5..UD7` are never emitted.
* **`SDX` (`0x0B`) and `UDX` (`0xF1`) are never emitted.** Every offset that needs defining is
  expressible with a one-byte window definition, because the ranges that are not expressible are
  exactly the ones classified *unencodable* and handled with SCU/Unicode mode instead.
* `UQU` (`0xF0`) is emitted; `SQU` (`0x0E`) is emitted; `SCU` (`0x0F`) is emitted.

### 2.6 Output budget

The encoder is given a budget of `2 * n` bytes for an *n*-character text and stops producing
output the moment the budget is reached, even mid-character. The caller then sees
`produced >= 2 * n` and stores the text raw (section 1.1), so a truncated encoding never reaches
the output file.

### 2.7 Verified byte examples

| characters | compressed bytes | notes |
|---|---|---|
| `Hello` | `48 65 6c 6c 6f` | all plain |
| `A` `U+0431` `B` | `41 03 b1 42` | lone Cyrillic character quoted: SQ2 + `0x80 + 0x31` |
| `П р и в е т` (`U+041F U+0440 U+0438 U+0432 U+0435 U+0442`) | `12 9f c0 b8 b2 b5 c2` | run of 6 in one class: SC2 then window-relative bytes |
| `A` `A` `\f` `B` `B` (`\f` = `U+000C`) | `41 41 01 0c 42 42` | lone C0 control quoted with SQ0 |
| `U+00E9 U+0431` (from a UTF-8 source) | `e9 03 b1` | `0xE9` is in the initial active window `0x0080` |
| `U+0080 U+0091 U+009E U+00E9` (Latin-1 source) | `80 91 9e e9` | all in the active window |
| 200 × `X` | `58` × 200, run header `80 c8` | two-byte run header |

The Cyrillic example above is the one quoted in the task; it is reproduced here from a real run:
`RESOURCE ... { a = <0x041f><0x0440><0x0438><0x0432><0x0435><0x0442>; }`.

---

## 3. Text member layout

With `-u` in force, `BUF`, `TEXT` and `LTEXT` are 16-bit types (`BUF16`, `TEXT16`, `LTEXT16`);
`BUF8`, `TEXT8` and `LTEXT8` are always 8-bit. Without `-u` the unsuffixed names are the 8-bit
forms; that mode is out of scope here.

### 3.1 The six forms

| type | layout |
|---|---|
| `BUF8` | the characters as bytes. No prefix, no terminator. |
| `LTEXT8` | one byte holding the character count, then the characters as bytes. |
| `TEXT8` | the characters as bytes, then one `0x00` byte. |
| `BUF16` | the text (compressible, alignable). No prefix, no terminator. |
| `LTEXT16` | one *raw* byte holding the character count, then the text. |
| `TEXT16` | the text with one extra `U+0000` appended to it — the terminator is part of the text and therefore part of the compressed run when the text is compressed. |

For `LTEXT16` the length byte is written as a plain byte into the resource image *before* the
text's alignment point, so it is never part of a compressed run, it counts towards `B`, and it is
what usually makes the text start at an odd offset. The count written is the character count
truncated to 8 bits; no check is made at this point.

8-bit text is never compressed and never aligned. Verified: `LTEXT8 a = "Hello"` gives
`05 48 65 6c 6c 6f` with the packed bit clear; `BUF8 a = "Hello"` gives `48 65 6c 6c 6f`.

### 3.2 `TEXT` / `TEXT8` / `TEXT16` are unusable in this build

**This build of `rcomp` crashes (access violation) whenever a `TEXT`, `TEXT8` or `TEXT16` member
is declared**, before writing any output. The crash happens while it tries to print the
"deprecated zero-terminated text" warning for that declaration. Consequently no golden file can
ever contain a `TEXT*` member, and the layout in the table above is derived from the
disassembly only, not verified against output bytes.

Recommendation for the reimplementation: implement the layout as described, and treat the
inability to produce a golden as expected rather than as a missing test.

### 3.3 Length limits on `BUF` and `LTEXT`

The declaration forms are `BUF name(n)` and `LTEXT name(n)` (parentheses, not brackets; `BUF<n>`
is not valid syntax in this grammar). Verified: `BUF a(8)` with `a = "Hi"` compiles and produces
a 2-character text — **the limit does not pad**; it is only an upper bound.

When the assigned text is longer than `n`, `rcomp` tries to report "text length exceeds specified
limit" and **crashes** in the same way as section 3.2, producing no output. So overflow
behaviour cannot be observed, and there is no golden for it. From the disassembly, the check is
a diagnostic only; it does not alter the bytes that would otherwise be written. **Unknown**:
whether the intended behaviour is truncation or abort — the real binary neither truncates nor
exits cleanly.

The same applies to `LTEXT8 a(n)`.

### 3.4 Text at the very start of a resource

If the first compressible text begins at offset 0 of the resource image and nothing (not even a
pending length prefix) precedes it, the leading empty compressed run is **not** inserted; the
image starts with that text's compressed run header.

Verified: `STRUCT AA { BUF a; }`, `a = "Hello"` → `05 48 65 6c 6c 6f`.

### 3.5 Text at the very end of a resource

The trailing raw-run header is suppressed only for the last compressible text and only when no
raw bytes follow it (`T == 0`). Otherwise the header is written even if the run itself is the
last thing in the resource.

Verified: `STRUCT AA { BUF a; WORD b; }`, `a="Hello"`, `b=0x1234` →
`05 48 65 6c 6c 6f 02 34 12` (compressed run 5, then raw run header `02`, then the `WORD`).

### 3.6 Alignment

Every 16-bit text that is stored **raw** carries an alignment point. At emit time, one `0xAB`
byte is written at that point if and only if the current offset **in the uncompressed image**
(the running B of section 1.3) is odd. The pad therefore aligns the text to an even offset
relative to the **start of the resource image**, not to the file, not to the enclosing struct.

The pad byte is `0xAB` in every form — packed resource, unpacked resource, top level, inside an
embedded struct, inside an array. `0x00` is never used as a text alignment pad.

Compressed texts have no alignment point, so no pad is emitted for them in the packed image; the
uncompressed-image accounting still includes the pad they would have needed, which is why the
uncompressed size can exceed the packed size by more than the byte savings alone.

Verified: `STRUCT AA { BYTE b; BUF a; }` with `b=0x12`, `a=<0x0431>` → `12 ab 31 04`, packed bit
clear, largest uncompressed 4.

### 3.7 Adjacent text members

Nothing special happens between two adjacent compressible texts except that the raw run between
them has length 0 (section 1.5). If the first of the two is stored raw and the second compressed,
the alignment of the second is still computed from the uncompressed offset after the first.

Verified: `STRUCT AA { LTEXT a; LTEXT b; }`, `a="Hello"`, `b="World"` →
`00 01 05 05 48 65 6c 6c 6f 01 05 05 57 6f 72 6c 64`
— empty compressed run, raw run `05` (the first length byte), compressed "Hello", raw run `05`
(the second length byte), compressed "World".

### 3.8 Arrays of text

An array member emits its element count first (section 5.3) and then the elements back to back;
each element is laid out exactly as the scalar form of that type. Nothing in the array machinery
interacts with text packing.

Verified: `STRUCT AA { LTEXT a[]; }`, `a = {"Hi","Yo"}` →
`00 03 02 00 02 02 48 69 01 02 02 59 6f`
— empty compressed run, raw run of 3 (`02 00` element count `WORD`, then the first `02` length
byte), compressed `48 69`, raw run of 1 (the second `02` length byte), compressed `59 6f`.

### 3.9 `U+0000` written explicitly in the source

The `<0x0000>` escape is broken in this build: depending on where it appears it is silently
dropped, or it turns into an unrelated character taken from uninitialised memory. Observed on
four runs:

| source | resulting text |
|---|---|
| `<0x0000> "AB"` | length 2, `41 42` — dropped |
| `"AB" <0x0000> "CD"` | length 4, `41 42 43 44` — dropped |
| `"AB" <0x0000>` | length 3, `41 42 0e 07 14` — became `U+0714` |
| `<0x0000>` alone | length 1, raw `cc 3c` — became `U+3CCC` |

Do not try to reproduce this. `U+0000` arriving through the normal channel (the `TEXT16`
terminator) is a plain character and is classified as *plain*, so it would encode as the single
byte `0x00` — but see section 3.2, that path cannot be exercised.

---

## 4. Confirmation of the already-derived items

All of these were re-verified against the real binary. Everything in the task's item 4 is
correct; the notes below add detail.

**Header.** `UID1` `0x101F4A6B`, `UID2`, `UID3`, then the UID checksum word, then the flags byte,
then the `u16` largest-uncompressed-resource size, then the packed-bit array, then the resource
images, then the index, then a final `u16` giving the index's file offset.

**Flags byte.** Bit 0 (`0x01`) is set when the source contains no `UID3` statement, and in that
case `UID3` in the header is the `NAME` value (or 0 if there is no `NAME` either). With an
explicit `UID3` statement the flags byte is `0x00`. Verified:

| source | flags | header UID3 |
|---|---|---|
| no `NAME`, no `UID3` | `0x01` | `0x00000000` |
| `NAME ABCD` | `0x01` | `0x000052EA` |
| `UID2 0x10004711` + `UID3 0x12345678` | `0x00` | `0x12345678` |

**Largest uncompressed resource.** The maximum, over **all** resources including unpacked ones,
of the uncompressed image size (section 0). For a resource with no 16-bit text this equals its
actual byte length.

**Packed-bit array.** One bit per resource, resource *i* (0-based) at bit `i mod 8` of byte
`i div 8`, least significant bit first, written out one byte at a time as soon as 8 bits are
accumulated; a final partial byte is written only if `count mod 8 != 0`. With zero resources no
byte is written at all. Verified: 8 text resources → one byte `ff`; 9 → two bytes `ff 01`.

**Resource id.** `(NAME value << 12) | index`, with `index` starting at 1 for the first resource
and incrementing for every resource, named or not. An `OFFSET n` statement sets the counter so
that the next resource gets index `n + 1`. Verified: `NAME ABCD` → ids `0x052EA001`,
`0x052EA002`; with `OFFSET 0x100` before the second resource its id is 257.

**`NAME` base 27.** Value starts at 0; for each character (at most four, longer is an error)
value is multiplied by 27 and, if the character is a letter, `toupper(c) - 0x40` is added
(so `A`/`a` = 1 … `Z`/`z` = 26); non-letters add nothing. Verified: `ABCD` → `0x52EA` (21226),
`a0z9` → `0x4FA1` (20385).

**`.rsg` lines.** For each resource that has a name and is not `LOCAL`, one line. The name is
upper-cased. The line is `#define `, then the name left-justified in a 41-character field, then
one space, then the value. When the source has a `NAME` statement the value is printed as
`0x` followed by lowercase hexadecimal with no leading zeros; otherwise as a decimal integer.
The value therefore starts at column 51 (offset 50) unless the name is 41 characters or longer,
in which case exactly one space separates them. Lines are terminated with CRLF. Unnamed
resources produce no line but still consume an index. Verified:

```
#define R1                                        1
#define R1                                        0x52ea001
#define R_THIS_IS_A_VERY_LONG_RESOURCE_NAME_OVER_41CH 1
```

**Undefined identifier where text is expected.** Confirmed as the stated behaviour in the golden
corpus, but note that the direct black-box test (`a = SomeUndefinedThing;` for an `LTEXT`)
**crashes** this build rather than substituting the spelling, so it cannot be demonstrated
in isolation. The same crash occurs for `LINK`/`LLINK` values naming a resource, and for
`rls_*` symbols referenced from a resource item. **Unknown** whether the corpus behaviour comes
from a different code path (for example the identifier being resolved by the preprocessor) or
from a state that the minimal test cases do not reach.

**`\f` is `U+000C`.** Verified: `"AA\fBB"` compresses to `41 41 01 0c 42 42`.

**Array counts.** Verified below in section 5.3.

**Unassigned members.** Verified: `STRUCT AA { BYTE b; WORD w; LONG l; LTEXT t; BUF u; LINK k;
LLINK m; }` with an empty resource body gives eight zero bytes — `BYTE` 1, `WORD` 2, `LONG` 4,
`LTEXT` its zero length byte, and **nothing at all** for `BUF`, `LINK` and `LLINK`. See 5.4.

---

## 5. Other things that change output bytes

### 5.1 Integer truncation and range

`BYTE`, `WORD` and `LONG` are written little-endian, truncated to 1, 2 and 4 bytes.

Negative values are accepted and written in two's complement. Values above the unsigned maximum
of the type are a hard error — and the error path **crashes** this build, producing no output.

Verified:

| declaration and value | bytes |
|---|---|
| `BYTE a = 255` | `ff` |
| `BYTE a = -1` | `ff` |
| `BYTE a = -128` | `80` |
| `BYTE a = 256` | crash, no output |
| `BYTE a = -129` | crash, no output |
| `WORD a = -2` | `fe ff` |
| `WORD a = 65536` | crash, no output |
| `WORD a = -32769` | crash, no output |
| `LONG a = -1` | `ff ff ff ff` |

So the accepted range is `-2^(8k-1) .. 2^(8k)-1` for a `k`-byte type, and anything outside it
aborts. No silent truncation of out-of-range literals happens in practice.

### 5.2 `DOUBLE`

IEEE-754 binary64, little-endian, 8 bytes. Integer literals are converted to double. Verified:

| value | bytes |
|---|---|
| `1.5` | `00 00 00 00 00 00 f8 3f` |
| `-0.1` | `9a 99 99 99 99 99 b9 bf` |
| `3` | `00 00 00 00 00 00 08 40` |

### 5.3 Arrays

| declaration | element-count prefix |
|---|---|
| `T name[]` | `WORD`, the number of elements actually supplied |
| `LEN BYTE T name[]` | one byte |
| `LEN WORD T name[]` | `WORD` |
| `T name[k]` (fixed size) | none |

The same applies to `STRUCT name[]` arrays. The count prefix is written as plain bytes into the
resource image, so it is always part of a raw run.

Verified: `WORD a[]` with `{1,2,3}` → `03 00 01 00 02 00 03 00`; `LEN BYTE WORD a[]` →
`03 01 00 02 00 03 00`; `WORD a[3]` → `01 00 02 00 03 00`; an unassigned `WORD a[]` → `00 00`;
`STRUCT s[]` with two one-`WORD` elements → `02 00 01 00 02 00`; `LEN BYTE STRUCT s[]` with one
element → `01 01 00`.

Note: a fixed-size array supplied with fewer initialisers than its declared size emits only the
supplied elements (verified: `WORD a[3]` with `{1,2}` → `01 00 02 00`, four bytes). Whether that
is intended is **unknown**; the diagnostic for it does not appear on stdout.

### 5.4 `LINK`, `LLINK`, `SRLINK`

* `LINK` is a `WORD`, `LLINK` is a `LONG`, `SRLINK` is a `LONG`.
* `SRLINK` is filled with the **id of the resource that contains it**. Verified: with two
  resources each `{ SRLINK s; WORD w; }`, resource 1 gives `01 00 00 00 01 00` and resource 2
  gives `02 00 00 00 02 00`.
* `LINK` and `LLINK` with a numeric value emit that value truncated to 2 / 4 bytes. Verified:
  `k=0x1234; m=0x12345678` → `34 12 78 56 34 12`.
* A `LINK` or `LLINK` **with no value at all emits zero bytes** — not two or four zero bytes.
  This is why struct declarations in the SDK headers always give them an explicit
  `= 0` default. Verified in the unassigned-members case in section 4.
* A `LINK`/`LLINK` whose value names a resource crashes this build in the minimal test case
  (see the note in section 4); the golden corpus resolves such references to the target
  resource's id.

### 5.5 Length prefix on a nested item list

An item list that is **nested** (that is, the member list of an embedded struct, not the
top-level member list of the resource) and that carries a declared length type emits a length
placeholder of that type (`BYTE`, `WORD` or `LONG`) before its contents, and the placeholder is
filled in afterwards with the number of **uncompressed-image** bytes the list occupies — not the
packed byte count, and not the element count. The placeholder occupies its own space in both
images and advances the alignment counter B.

The top-level member list of a resource never gets such a prefix even if a length type is
declared, because the nesting depth is zero there.

A plain `STRUCT name;` scalar member has no prefix at all: its contents are spliced into the
enclosing resource. Verified: `STRUCT BB { WORD x; } STRUCT AA { STRUCT s; }` with
`s = BB { x=0x1234; }` → `34 12`.

**Unknown**: the exact source syntax that attaches a length type to a nested list. Every
spelling tried (`STRUCT BYTE s;`, `STRUCT WORD s;`, `LEN BYTE STRUCT s;`, `LEN WORD STRUCT s;`
for a scalar member) is a syntax error, and this build crashes on syntax errors instead of
printing a diagnostic. The golden corpus contains no example either. If the implementer can
find the syntax, the semantics above are what to implement.

### 5.6 `CHARACTER_SET` and the default source character set

The recognised names are `ASCII`, `CP1252`, `CP850`, `ISOLATIN1`, `SHIFTJIS`, `UNICODE`, `UTF8`.

* The **default**, with no `CHARACTER_SET` statement, is **CP1252**. Verified: source bytes
  `80 91 9E E9` produce the characters `U+20AC U+2018 U+017E U+00E9`, which is exactly the
  CP1252 mapping.
* `ISOLATIN1`, `ASCII` and `CP850` all behave identically to ISO 8859-1: source byte *b* becomes
  `U+00`*b*. Verified: the same source bytes give `U+0080 U+0091 U+009E U+00E9` under all three.
  `ASCII` does not reject high bytes and `CP850` does not apply the CP850 table.
* `UTF8` decodes UTF-8. Verified: `C3 A9 D0 B1` → `U+00E9 U+0431`.
* `UNICODE` is rejected ("Unicode source is unsupported"), and the rejection path **crashes** this
  build.
* An unrecognised name produces a warning and falls back to "no character set" — and that warning
  path also **crashes** this build.
* `SHIFTJIS` was not tested. **Unknown**.

### 5.7 `rls_*` statements

`rls_string`, `rls_string8`, `rls_byte`, `rls_word`, `rls_long`, `rls_double` are accepted as
statements (`rls_string NAME "text"` etc.) and appear throughout the golden corpus' preprocessed
sources. Referencing such a symbol as a resource item value crashes this build in the minimal
test case, so their effect on output bytes could not be characterised black-box. In the golden
corpus they are localisation declarations that carry no bytes of their own. **Unknown**: whether
a reference to an `rls_*` symbol substitutes its value or falls through to the
undefined-identifier behaviour.

### 5.8 Crashing diagnostics — summary

This build reliably crashes (access violation, no output file) instead of emitting a diagnostic in
at least these situations. None of them can appear in the golden corpus, so the reimplementation
has no golden to match and is free to report a clean error instead:

* any `TEXT`, `TEXT8` or `TEXT16` member declaration (deprecation warning);
* a text longer than a declared `(n)` limit;
* an integer literal outside the accepted range of its type;
* an unrecognised `CHARACTER_SET` name, and `CHARACTER_SET UNICODE`;
* any syntax error;
* a `LINK`/`LLINK` or text value naming an unresolved identifier or an `rls_*` symbol, at least in
  small standalone inputs.

---

## 6. Test vectors

Each row is a complete `.rss` input (with `-u`) and the resulting resource image. Header, index
and `.rsg` are omitted except where stated.

| # | source | resource bytes | packed | largest uncompressed |
|---|---|---|---|---|
| 1 | `STRUCT AA { LTEXT a; } RESOURCE AA r1 { a = "Hello"; }` | `00 01 05 05 48 65 6c 6c 6f` | yes | 12 |
| 2 | `STRUCT AA { BUF a; } RESOURCE AA r1 { a = "Hello"; }` | `05 48 65 6c 6c 6f` | yes | 10 |
| 3 | `STRUCT AA { LTEXT8 a; } RESOURCE AA r1 { a = "Hello"; }` | `05 48 65 6c 6c 6f` | no | 6 |
| 4 | `STRUCT AA { BUF8 a; } RESOURCE AA r1 { a = "Hello"; }` | `48 65 6c 6c 6f` | no | 5 |
| 5 | `STRUCT AA { BUF a; WORD b; } RESOURCE AA r1 { a = "Hi"; b=7; }` | `48 00 69 00 07 00` | no | 6 |
| 6 | `STRUCT AA { BYTE b; BUF a; } RESOURCE AA r1 { b=0x12; a="Hello"; }` | `00 01 12 05 48 65 6c 6c 6f` | yes | 12 |
| 7 | `STRUCT AA { BYTE b; BUF a; } RESOURCE AA r1 { b=0x12; a=<0x0431>; }` | `12 ab 31 04` | no | 4 |
| 8 | `STRUCT AA { BUF a; BUF b; } RESOURCE AA r1 { a="Hello"; b="World"; }` | `05 48 65 6c 6c 6f 00 05 57 6f 72 6c 64` | yes | 20 |
| 9 | `STRUCT AA { BUF a; WORD b; } RESOURCE AA r1 { a="Hello"; b=0x1234; }` | `05 48 65 6c 6c 6f 02 34 12` | yes | 12 |
| 10 | `STRUCT AA { WORD b; BUF a; } RESOURCE AA r1 { b=0x1234; a="Hello"; }` | `00 02 34 12 05 48 65 6c 6c 6f` | yes | 12 |
| 11 | `STRUCT AA { LTEXT a; LTEXT b; } RESOURCE AA r1 { a="Hello"; b="World"; }` | `00 01 05 05 48 65 6c 6c 6f 01 05 05 57 6f 72 6c 64` | yes | 24 |
| 12 | `STRUCT AA { LTEXT a[]; } RESOURCE AA r1 { a = {"Hi","Yo"}; }` | `00 03 02 00 02 02 48 69 01 02 02 59 6f` | yes | 14 |
| 13 | `STRUCT AA { LTEXT a[]; } RESOURCE AA r1 { a = {"2","2","2","2","2","2","2","2","2"}; }` | see section 1.6 | yes | 38 |
| 14 | `STRUCT AA { LTEXT a; } RESOURCE AA r1 { a = "A" <0x0431> "B"; }` | `00 01 03 04 41 03 b1 42` | yes | 8 |
| 15 | `STRUCT AA { LTEXT a; } RESOURCE AA r1 { a = "AA\fBB"; }` | `00 01 05 06 41 41 01 0c 42 42` | yes | 12 |
| 16 | `STRUCT AA { LTEXT a; } RESOURCE AA r1 { a = <0x0431>; }` | `01 ab 31 04` | no | 4 |
| 17 | `STRUCT AA { BYTE b; WORD w; LONG l; LTEXT t; BUF u; LINK k; LLINK m; } RESOURCE AA r1 { }` | `00 00 00 00 00 00 00 00` | no | 8 |
| 18 | `STRUCT AA { SRLINK s; WORD w; } RESOURCE AA r1 { w=1; } RESOURCE AA r2 { w=2; }` | `01 00 00 00 01 00` / `02 00 00 00 02 00` | no | 6 |
| 19 | `STRUCT AA { DOUBLE a; } RESOURCE AA r1 { a=1.5; }` | `00 00 00 00 00 00 f8 3f` | no | 8 |
| 20 | `CHARACTER_SET ISOLATIN1` + `STRUCT AA { LTEXT a; } RESOURCE AA r1 { a = "\x80\x91\x9e\xe9"; }` | `00 01 04 04 80 91 9e e9` | yes | 10 |
| 21 | `STRUCT AA { LTEXT a; } RESOURCE AA r1 { a = "\x80\x91\x9e\xe9"; }` (default CP1252) | `04 ab ac 20 18 20 7e 01 e9 00` | no | 10 |

Full-file check for row 1 (`-u`, no `NAME`, no `UID3`):

```
6b 4a 1f 10  00 00 00 00  00 00 00 00  19 fd 48 e8
01 0c 00 01  00 01 05 05  48 65 6c 6c  6f 14 00 1d
00
```

— UID1, UID2 = 0, UID3 = 0, checksum, flags `01`, largest uncompressed `0c 00`, packed bits `01`,
resource image at offset `0x14`, index (`14 00`) at offset `0x1D`, trailing index offset `1d 00`.
The `.rsg` is `#define R1` padded to column 51 then `1`, terminated with CRLF.
