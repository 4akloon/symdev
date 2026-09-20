# `bmconv` — behavioural specification for a native reimplementation

Written 2026-09-20 by a separate agent (the "spec writer") from disassembly of
`/home/genius/sdk/S60_3rd_FP2/epoc32/tools/bmconv.exe` (the S60 3rd Edition FP2 build, which
identifies itself as *BMCONV version 112*) combined with black-box runs of that binary under Wine.
The repository owner approved disassembling the SDK tool for this purpose. The engineer who
implements the native replacement did not see the disassembly, any decompiler output, or any
scratch notes; this document is the only handover artefact. Every numeric rule below was
re-confirmed by running the real tool on crafted input and comparing output bytes, and the byte
examples in §14 were produced that way. Where a claim could not be confirmed it is marked
**unknown**.

All byte examples were produced with Windows-style paths (`Z:\...`) under Wine. All multi-byte
integers in every file format described here are **little-endian**, and every count/size/offset
field is a signed 32-bit integer unless stated otherwise.

---

## 1. What the tool does

`bmconv` compiles one or more Windows `.bmp` files into a Symbian **multi-bitmap** file (`.mbm`),
optionally emitting a C++ header (`.mbg`) with an enumeration naming each bitmap. It can also
produce two ROM-image variants of the same container, decompile an existing container back to
`.bmp` files, and print a summary of a container's contents.

Three container flavours exist, selected on the command line:

| Flavour | Selected by | Container first word |
|---|---|---|
| File store (default) | no flag | `0x10000037` |
| ROM image store | `/r` | `0x10000041` |
| Compressed ROM image store | `/s` | `0x10000041` |

`/r` and `/s` produce the same container layout; they differ only in whether per-bitmap
compression is applied.

---

## 2. Command line

### 2.1 Usage text

Run with no arguments the tool prints the following and exits with code 0. This is the complete,
verbatim usage text (leading blank lines and the version banner shown for completeness):

```

BMCONV version 112.
Symbian OS multiple bitmap file/rom store conversion program.
Copyright (c) 1998-2001 Symbian Ltd.  All rights reserved.

Usage:
BMCONV [/r|/s|/n] [/hfilename] [/q] [/pfilename] epocfile [OPT]bmp_1 ... [OPT]bmp_n
BMCONV [/r|/s|/n] [/q] [/pfilename] epocfile /mepocfile2
BMCONV /u epocfile bmp_1 [... bmp_n]
BMCONV /v epocfile
BMCONV commandfile

 /r specifies a ROM image destination file,
 /s specifies a compressed ROM image file,
 /n disables bitmap File Store compression,
 the default is a compressed File Store file.

 /q specifies quiet mode - only errors are reported.

 /hfilename specifies the filename for the automatic
 generation of a header file for inclusion into code.

 /pfilename gives the filename of a palette file containing 256 hex
 numbers (0x00BBGGRR) specifying the palette for 8bpp colour bitmaps.
 (Omission results in the use of a default palette.)

 OPT may be one of /1, /2, /4, /8, /c4, /c8, /c12, /c16, /c24
 specifying bits per pixel and grey-scale/colour, or /mepocfile2
 to specify an existing multiple bitmap file. default is /2.

 epocfile specifies the epoc multi-bitmap file name.
 bmp_n specifies the nth bitmap file name.

 /u decompiles epocfile to bmp_1,...,bmp_n
 /v displays a summary of the bitmaps in epocfile
 otherwise bmp_1,...,bmp_n are compiled to epocfile

 commandfile specifies a file containing the commandline
 with commands separated by spaces or newlines.
```

### 2.2 Global options

Global options must appear **before** the destination file name. The tool scans arguments from the
first one and stops at the first argument that does not begin with `/`. Anything after that point
is treated as the destination followed by the source list.

| Option | Effect |
|---|---|
| `/r` | ROM image store output. Bitmaps are never compressed. |
| `/s` | Compressed ROM image store output. Bitmaps are always compressed (`/n` is ignored). |
| `/n` | For the default file store only: disable per-bitmap compression. |
| `/q` | Quiet. On success nothing at all is printed (verified: empty stdout). Errors are still printed. |
| `/h`*name* | Write a C++ header file. The name is the rest of the same argument, with no space. |
| `/p`*name* | Read a 256-entry palette file. The name is the rest of the same argument, with no space. |

Recognition details, all verified:

* The letter after `/` is matched case-insensitively for the **global** options (`/R`, `/S`, `/N`,
  `/H`, `/Q`, `/P` all work).
* `/r` together with `/s` (in either order) is an error: exit code 8, message
  `Too many arguments.`
* If `/h` or `/p` appears more than once the last occurrence wins.
* An unrecognised `/x` option is **not** consumed. It stays in the argument list and therefore
  becomes the destination file name, which usually then fails to open. Example: running with `/z`
  as the first argument prints `Epoc file: /z` and then `Bad destination file(s).` with exit
  code 5.
* A global option placed **after** the destination is treated as a per-bitmap option on a source
  file. For example `/n` in that position is parsed as bitmap mode `'n' - '0'` = 30, which is
  invalid: exit code 12, `Invalid bitmap mode specified.`

### 2.3 Per-bitmap options

Each source argument may carry a depth option glued directly to the front of the file name, e.g.
`/c8Z:\path\icon.bmp`. The parse is:

1. If the argument does not start with `/`, the default mode is used: **2 bpp greyscale**.
2. Otherwise skip the `/`.
3. If the next character is a lower-case `c`, the bitmap is a *colour* bitmap and the character is
   consumed. Upper-case `C` is **not** recognised here (verified: `/C8` is rejected with exit
   code 12 and the residual text `8` is prepended to the file name in the diagnostic).
4. The next character is read as a decimal digit, giving a value `v`.
5. The character after that is inspected. Only the digits `2`, `4` and `6` are accepted as a
   second digit; if present it is consumed and `v` becomes `v × 10 + d`. Any other character (the
   first character of the file name, normally a drive letter) terminates the number.
6. The remainder of the argument is the file name.

Accepted combinations:

| Option | Target bpp | Colour flag in header | Notes |
|---|---|---|---|
| `/1` | 1 | 0 | greyscale (monochrome) |
| `/2` | 2 | 0 | greyscale — the default when no option is given |
| `/4` | 4 | 0 | greyscale |
| `/8` | 8 | 0 | greyscale |
| `/c4` | 4 | 1 | 16-colour palette |
| `/c8` | 8 | 1 | 256-colour palette |
| `/c12` | 12 | 1 | 4-4-4 direct colour |
| `/c16` | 16 | 1 | 5-6-5 direct colour |
| `/c24` | 24 | 1 | 8-8-8 direct colour |

Everything else is rejected with exit code 12, `Invalid bitmap mode specified.` Verified
rejections: `/3`, `/16`, `/c1`, `/c2`, `/c22`, `/c26`, `/c14`, `/c32`, `/C8`.

Note the consequence of step 5: a two-digit mode whose second digit is not 2, 4 or 6 cannot be
expressed at all, and `/c14`, `/c22`, `/c26` parse successfully as numbers but then fail
validation.

### 2.4 `/m` — reuse an existing container

`/m`*existing.mbm* may be given **as the only source argument**. The tool then copies every bitmap
out of that container into the new destination, re-applying the destination's container flavour and
compression setting. Verified: copying a two-bitmap file store to a new file store with default
settings reproduces the input file byte for byte.

* If `/m` is combined with any other source argument, the `/m` token is parsed as a per-bitmap
  option instead (mode `'m' - '0'` = 29) and the run fails with exit code 12.
* If `/h` is also given, the tool prints `Header file generation is not permitted with /m` and
  continues without writing a header; the exit code is still 0.
* The referenced container may be either flavour; the first word decides (`0x10000037` = file
  store, `0x10000041` = ROM image).

### 2.5 `/u` — decompile

`BMCONV /u epocfile bmp_1 [... bmp_n]` writes bitmap *i* of the container to the *i*-th named
output file. Fewer output names than bitmaps is allowed (only that many are written, exit code 0).
Naming more outputs than there are bitmaps fails with exit code 7.

Every output is a **24-bit bottom-up BMP** with a 40-byte info header, regardless of the stored
depth, built as follows:

| Field | Value |
|---|---|
| signature | `BM` |
| file size | 54 + row-padded data size |
| reserved words | 0, 0 |
| pixel data offset | 54 (`0x36`) |
| info header size | 40 (`0x28`) |
| width, height | the bitmap's pixel width and height (height positive) |
| planes | 1 |
| bit count | 24 |
| compression | 0 |
| image size, x/y pixels-per-metre, colours used, colours important | all 0 |

Row stride is `((width × 3) + 3) & ~3`. Row padding bytes are **`0xFF`**, not zero. Because
pixels-per-metre is written as zero, recompiling a `/u` output yields a bitmap with twips size 0×0
(see §5.2).

### 2.6 `/v` — summary

`BMCONV /v epocfile` prints, for the whole file, `<name> is a <File store|ROM image> containing
<n> bitmap[s]`, then for each bitmap:

```
Bitmap <i> information:
Pixel size <w> x <h>
Twips size <wt> x <ht>
<bpp> Bpp <Colour|Monochrome>
<compression description>
```

`Palette entries <n>` is printed between the Bpp line and the compression line only when the
stored palette-entry count is greater than zero (this never happens for files the tool itself
writes). The compression descriptions are `No compression`, `Bytewise RLE compression <n>%`,
`12 bit RLE compression <n>%`, `16 bit RLE compression <n>%`, `24 bit RLE compression <n>%`, where
`<n>` is `(bitmapSize × 100 − 4000) ÷ (stride × height)`, integer division, and 0 when the
denominator is not positive. `/v` on a non-container file prints `Bad source file(s).` (exit code
is 0 for `/v` in the observed runs).

### 2.7 Command files

If exactly one argument is given and it does not start with `/u` or `/v`, it is treated as a
**command file**: the whole file is read and split into arguments.

* A `//` sequence begins a comment. From that point every character is replaced by a space until
  a carriage return or a line feed is reached (the comment terminator is consumed as part of the
  scan).
* Byte `0x1A` is replaced by a space.
* Tokens are separated by space, line feed, carriage return, or `0x1A`.
* An unreadable or empty command file yields exit code 6, `Bad command file.`

Verified: a file containing a `//` comment line, then the destination name, then `/c8<source>`,
each on its own line, compiles correctly.

### 2.8 Exit codes

The process exit code is the internal result code. The full mapping (message ⇄ code) is:

| Code | Message |
|---|---|
| 0 | `Success.` |
| 1 | `Out of memory.` |
| 2 | `Bad argument.` |
| 3 | `File does not exist` (no full stop) |
| 4 | `Bad source file(s).` |
| 5 | `Bad destination file(s).` |
| 6 | `Bad command file.` |
| 7 | `Number of sources/targets mismatch.` |
| 8 | `Too many arguments.` |
| 9 | `Unknown source compression type.` |
| 10 | `Compression error.` |
| 11 | `Decompression error.` |
| 12 | `Invalid bitmap mode specified.` |
| 13 | `Bad palette file.` |
| 14 | `Palettes not supported` (no full stop) |
| other | `Unknown error!` |

Verified exit codes: 0 (success and for the no-argument usage print), 3 (source file missing),
4 (source is not a usable BMP), 5 (destination or header file cannot be created), 6 (palette file
missing — note the misleading message), 7 (destination given with no sources), 8 (`/r` with `/s`),
9 (BMP with a non-zero compression field), 12 (bad depth option), 13 (palette file with fewer than
256 entries).

### 2.9 Console output on a successful compile

Unless `/q` is given, the tool prints two blank lines, the version banner, `Compiling...`, the
store type line, `Epoc file: <name>`, a blank line, one `Bitmap file <i>\t: <name>` line per
source, and finally `Success.` Line endings are CRLF. For `/u` the second line is `Decompiling...`
instead. All output goes to standard output.

---

## 3. Reading the source BMP

### 3.1 Acceptance

1. The first two bytes must be `BM`, otherwise exit code 4.
2. Exactly 14 bytes of file header and then exactly 40 bytes of info header must be readable.
   A short read gives exit code 4. **Only the 40-byte `BITMAPINFOHEADER` form is supported**; a
   12-byte core header, or a 108/124-byte V4/V5 header, is not handled (the tool would read the
   extra header bytes as palette/pixel data).
3. The compression field of the info header must be 0. Any other value gives exit code 9,
   `Unknown source compression type.` (verified with value 1). So RLE4/RLE8/BITFIELDS sources are
   rejected.
4. The colour count is taken from the "colours used" field. If that field is zero and the bit
   count is neither 24 nor 32, the count becomes `1 << bitCount`; if the field is zero and the bit
   count is 24 or 32, the count is 0.
5. If the resulting colour count is greater than 256 the file is rejected with exit code 4.
   **Consequence:** a 16-bpp BMP with "colours used" = 0 yields `1 << 16` = 65536 and is always
   rejected (verified: exit code 4).
6. The palette is then read as `count × 4` bytes **immediately after the 54-byte header**. The
   "pixel data offset" field of the file header is completely ignored.
7. The pixel data length is computed as `bfSize − 54 − count × 4`, where `bfSize` is the *file
   size field of the BMP file header*, not the real file length. Exactly that many bytes must be
   readable; otherwise exit code 4. Verified: inflating `bfSize` by 100 makes the tool reject the
   file with exit code 4.

Byte example: a well-formed 4×1 8-bpp BMP with a 256-entry palette is accepted; the same file with
its `bfSize` field increased by 100 is rejected with exit code 4.

### 3.2 Pixel decoding by source depth

Source rows are read bottom-up: row *y* of the output (counted from the top) comes from source row
`height − y − 1`. Source stride in bytes, by source bit count:

| Source bpp | Stride |
|---|---|
| 1 | `((w + 31) ÷ 8) rounded down to a multiple of 4` (equals the usual `((w+31)/32)×4`) |
| 4 | `((w + 7) ÷ 2) rounded down to a multiple of 4` |
| 8 | `(w + 3) rounded down to a multiple of 4` |
| 16 | `(w × 2 + 2) rounded down to a multiple of 4` |
| 24 | `((w + 1) × 3) rounded down to a multiple of 4` |
| 32 | `w × 4` |

Each source pixel is turned into an (R, G, B) triple:

* **1 bpp** — the bit is taken most-significant-bit-first within its byte. If the BMP has a
  palette, bit 0 selects palette entry 0 and bit 1 selects palette entry 1. If there is no
  palette, 0 becomes black and 1 becomes white. Verified with a palette of red/blue: bits 0,1,0,1
  produce red, blue, red, blue.
* **4 bpp** — the byte at `x ÷ 2`; for even `x` the **high** nibble is used. With a palette the
  nibble indexes it. Without a palette the grey value `nibble × 0x11` is used for all three
  channels.
* **8 bpp** — the byte indexes the palette if there is one, otherwise it is used as a grey value
  for all three channels.
* **16 bpp** — **broken.** The tool builds the triple as R = low 5 bits of the 16-bit word,
  G = bit 5 of the word, B = 0. This is not RGB555 or RGB565 and produces nonsense colours. In
  practice 16-bpp sources are unreachable anyway because of the "colours used" check in §3.1.5,
  unless the BMP sets "colours used" to a value of 256 or less. Treat 16-bpp sources as
  unsupported.
* **24 bpp** — the three bytes are B, G, R in file order.
* **32 bpp** — the low three bytes of the 32-bit word are B, G, R; the fourth byte (alpha) is
  discarded.
* **Any other bit count (including 2)** — no pixel is read at all and the triple is left at its
  initial value, opaque **white**. Verified: a 2-bpp BMP with a four-colour palette compiles to an
  all-white bitmap with exit code 0 and no diagnostic.

If the BMP's palette is shorter than `2^bitCount` (i.e. "colours used" is set to a small value)
and a pixel index exceeds it, the tool indexes past the end of the allocated palette. **Unknown**:
the resulting bytes were not characterised; avoid such sources.

### 3.3 The two hard-coded colour substitutions

After the triple is built and **before** any depth conversion, two exact colours are replaced:

| Source colour | Replaced by |
|---|---|
| R=G=B=0x80 | R=G=B=0x7F |
| R=G=B=0xC0 | R=G=B=0xBB |

The test is an exact three-channel equality; nothing else is touched. Verified with a 6×1 24-bpp
row of `808080 C0C0C0 818181 BFBFBF 7F7F7F BBBBBB`:

* at `/8` the stored bytes are `7F BB 81 BF 7F BB` (plus padding),
* at `/c24` the stored bytes are `7F7F7F BBBBBB 818181 BFBFBF 7F7F7F BBBBBB` (plus padding).

This substitution is almost certainly there to dodge the Windows reserved palette entries. A
reimplementation must reproduce it or icon bytes will differ.

---

## 4. Converting a pixel to the target depth

Throughout this section (R, G, B) is the triple from §3.

### 4.1 Greyscale targets (`/1`, `/2`, `/4`, `/8`)

First compute an 8-bit grey level:

> grey = (2 × R + 5 × G + 1 × B) ÷ 8, truncated toward zero.

Then:

| Target | Stored value |
|---|---|
| `/8` | grey |
| `/4` | grey ÷ 16 (truncated) |
| `/2` | grey ÷ 64 (truncated) |
| `/1` | grey ÷ 128 (truncated) |

Verified byte examples (`/8`, no compression):

| (R, G, B) | grey | `/1` | `/2` | `/4` | `/8` |
|---|---|---|---|---|---|
| 0,0,0 | 0 | 0 | 0 | 0 | `00` |
| 255,255,255 | 255 | 1 | 3 | 15 | `FF` |
| 255,0,0 | 63 | 0 | 0 | 3 | `3F` |
| 0,255,0 | 159 | 1 | 2 | 9 | `9F` |
| 0,0,255 | 31 | 0 | 0 | 1 | `1F` |
| 48,32,16 | 34 | 0 | 0 | 2 | `22` |
| 96,80,64 | 82 | 0 | 1 | 5 | `52` |
| 3,2,1 | 2 | 0 | 0 | 0 | `02` |
| 9,8,7 | 8 | 0 | 0 | 0 | `08` |

### 4.2 Pixel and twips size

Pixel size is the BMP's width and height verbatim.

Twips size is derived from the BMP's pixels-per-metre fields. For the horizontal axis:

> if xPelsPerMetre ≤ 0 then 0, else ⌊ ⌊ width × 1 440 000 ÷ 254 ⌋ ÷ xPelsPerMetre ⌋

and likewise for the vertical axis with height and yPelsPerMetre. Both divisions truncate toward
zero and are done in that order (the inner one first). Note that this is a factor of ten away from
the physically correct conversion — the tool behaves as if the BMP field were pixels per decimetre.
Reproduce it as written.

Verified pairs (all with 2835 px/m, the usual 72 dpi value, unless noted):

| width | px/m | twips |
|---|---|---|
| 1 | 2835 | 1 |
| 4 | 2835 | 7 |
| 5 | 2835 | 9 |
| 10 | 2835 | 19 |
| 13 | 2835 | 25 |
| 24 | 2835 | 47 |
| 32 | 2835 | 63 |
| 100 | 2835 | 199 |
| 239 | 2835 | 477 |
| 1 | 5669 | 1 |
| 1 | 5670 | 0 |
| 10 | 1000 | 56 |
| 10 | 3780 | 14 |
| 10 | 5000 | 11 |
| 10 | 11811 | 4 |
| any | 0 | 0 |

The intermediate product overflows a signed 32-bit integer for widths above about 1491 pixels.
**Unknown**: the tool's behaviour in that range was not tested.

### 4.3 Colour targets

**`/c4` (16 colours).** The pixel is mapped through a fixed 512-entry lookup table indexed by

> index = (B ÷ 32) × 64 + (G ÷ 32) × 8 + (R ÷ 32)

i.e. the top three bits of each channel with blue most significant. The table value is the palette
index 0–15. The 16-colour palette (see §4.4) is fixed; `/p` does **not** affect `/c4` (verified:
identical output with and without a custom palette file).

**`/c8` (256 colours).** The pixel is mapped through a fixed 4096-entry lookup table indexed by

> index = (B ÷ 16) × 256 + (G ÷ 16) × 16 + (R ÷ 16)

i.e. the top four bits of each channel with blue most significant. Only the top nibbles matter, so
`0x3F3F3F` and `0x333333` map to the same index. When `/p` is given, a table built from the custom
palette replaces the built-in one (see §4.5).

**`/c12`.** Two bytes per pixel, stored little-endian, value

> (R ÷ 16) × 256 + (G ÷ 16) × 16 + (B ÷ 16)

i.e. `0x0RGB` with 4 bits per channel, truncating the low nibbles. Verified: red → `0x0F00`
(bytes `00 0F`), green → `0x00F0` (bytes `F0 00`), blue → `0x000F` (bytes `0F 00`), white →
`0x0FFF` (bytes `FF 0F`), and (R,G,B)=(48,32,16) → `0x0321` (bytes `21 03`).

**`/c16`.** Two bytes per pixel, stored little-endian, RGB 5-6-5:

> ((R AND 0xF8) × 256) + ((G AND 0xFC) × 8) + (B ÷ 8)

Verified: red → `0xF800` (bytes `00 F8`), green → `0x07E0` (bytes `E0 07`), blue → `0x001F`
(bytes `1F 00`), and (R,G,B)=(48,32,16) → `0x3102` (bytes `02 31`).

**`/c24`.** Three bytes per pixel in the order **B, G, R** — the same order as the 24-bpp BMP
source, so for a 24-bpp source it is a straight copy. Verified: source bytes `FF 00 00` (blue)
are stored as `FF 00 00`.

### 4.4 The two built-in palettes

Palette entries are 32-bit words of the form `0x00BBGGRR` — the same convention the `/p` file uses.

**16-colour palette** (used by `/c4`):

| Index | Word | (R, G, B) |
|---|---|---|
| 0 | `0x000000` | 0, 0, 0 |
| 1 | `0x555555` | 0x55, 0x55, 0x55 |
| 2 | `0x000080` | 0x80, 0, 0 |
| 3 | `0x008080` | 0x80, 0x80, 0 |
| 4 | `0x008000` | 0, 0x80, 0 |
| 5 | `0x0000FF` | 0xFF, 0, 0 |
| 6 | `0x00FFFF` | 0xFF, 0xFF, 0 |
| 7 | `0x00FF00` | 0, 0xFF, 0 |
| 8 | `0xFF00FF` | 0xFF, 0, 0xFF |
| 9 | `0xFF0000` | 0, 0, 0xFF |
| 10 | `0xFFFF00` | 0, 0xFF, 0xFF |
| 11 | `0x800080` | 0x80, 0, 0x80 |
| 12 | `0x800000` | 0, 0, 0x80 |
| 13 | `0x808000` | 0, 0x80, 0x80 |
| 14 | `0xAAAAAA` | 0xAA, 0xAA, 0xAA |
| 15 | `0xFFFFFF` | 0xFF, 0xFF, 0xFF |

**256-colour palette** (used by `/c8` when `/p` is absent). It is built from a 6×6×6 colour cube
plus 40 extra ramp entries. Let the six levels be `0x00, 0x33, 0x66, 0x99, 0xCC, 0xFF` and define
cube entry *k* (for *k* = 0…215) as the word with RR = level[*k* mod 6], GG = level[(*k* ÷ 6) mod 6],
BB = level[*k* ÷ 36]. Then:

* indices 0…107 are cube entries 0…107,
* indices 108…147 are the 40 extra entries listed below, in this order,
* indices 148…255 are cube entries 108…215.

The 40 extra entries, as words:

| Indices | Words |
|---|---|
| 108–112 | `111111 222222 444444 555555 777777` (greys) |
| 113–117 | `000011 000022 000044 000055 000077` (red ramp) |
| 118–122 | `001100 002200 004400 005500 007700` (green ramp) |
| 123–127 | `110000 220000 440000 550000 770000` (blue ramp) |
| 128–132 | `880000 AA0000 BB0000 DD0000 EE0000` (blue, high) |
| 133–137 | `008800 00AA00 00BB00 00DD00 00EE00` (green, high) |
| 138–142 | `000088 0000AA 0000BB 0000DD 0000EE` (red, high) |
| 143–147 | `888888 AAAAAA BBBBBB DDDDDD EEEEEE` (greys, high) |

This construction was checked entry by entry against the tool's table: all 256 words match.

Useful spot checks confirmed through the tool: pure red → index 5, pure green → index 30, pure
blue → index 220, black → 0, white → 255, and `0x7F7F7F` → index 112.

**Deriving the two lookup tables.** Both built-in tables are exactly the *nearest colour by
city-block distance* (sum of the absolute differences of the three channels), scanning palette
indices in increasing order, keeping the first index on a tie, and using channel representatives
`nibble × 0x11` for the 4096-entry table and `bits × 255 ÷ 7` for the 512-entry table. This was
verified for all 4096 and all 512 entries against the tool's tables. An implementation may
therefore generate both tables from the palettes rather than embedding them.

The tables can also be recovered purely black-box, which is the recommended way to build a golden
test: create a 64×64 24-bpp BMP whose pixel at (x, y) has B = ((y×64+x) ÷ 256 mod 16) × 0x11,
G = ((y×64+x) ÷ 16 mod 16) × 0x11, R = ((y×64+x) mod 16) × 0x11, and compile it with `/c8 /n`.
The 4096 stored bytes, read in row order, are exactly the 4096-entry table. Compiling the same
image with `/c4 /n` gives the 512-entry table collapsed onto 4-bit inputs. Both were confirmed to
match the tool's internal tables exactly.

### 4.5 `/p` — custom palette file

The file is scanned for the two-character sequence `0x` or `0X`. Each time it is found, **exactly
ten characters** are taken starting at that `0`, and the scan resumes immediately after them. The
ten characters are expected to be `0x` followed by eight hexadecimal digits; digits 1–2 are
ignored, digits 3–4 give BB, digits 5–6 give GG, digits 7–8 give RR. A character that is not a
hexadecimal digit contributes 0 (digits `0`–`9`, `a`–`f` and `A`–`F` are recognised; anything else
is zero).

Exactly 256 entries must be found; the scan runs off the end of the buffer otherwise and the run
fails with exit code 13, `Bad palette file.` (verified with a file containing 255 entries). A
palette file that cannot be opened gives exit code 6, `Bad command file.` (verified) — the message
is wrong but the code is what it is.

Once loaded, a 4096-entry inverse table is built with the same rule as §4.4: for each of the 4096
4-bit RGB triples, the representative colour is `nibble × 0x11` per channel, the metric is the sum
of absolute channel differences, palette entries are scanned from index 0 upwards, the first
smallest wins, and an exact match short-circuits the scan.

The custom palette affects `/c8` only. `/c4` and all greyscale modes are unchanged (verified). The
palette itself is **not** stored anywhere in the `.mbm` — the palette-entry field of every bitmap
header is always 0 — so a container built with `/p` is only meaningful to a consumer that knows the
same palette.

Verified byte example. With a palette file of 256 lines `0x00iiiiii` for *i* = 0…255 (that is, 256
greys), a 3×2 source whose top row is red, green, blue and whose bottom row is white, black,
`0x808080` compiles at `/c8 /n` to the data bytes `00 00 00 FF | FF 00 77 FF` (stride 4, padding
`FF`). The three saturated colours all collapse to grey 0; white → 255; black → 0;
`0x808080` → substituted to `0x7F7F7F` → quantised to nibble 7 → representative `0x77` → index 119
= `0x77`.

### 4.6 Row stride and padding in the stored bitmap

The stored row stride in bytes depends only on the target depth and the pixel width:

| Target bpp | Stride in bytes |
|---|---|
| 1 | `⌊(w + 31) ÷ 32⌋ × 4` |
| 2 | `⌊(w + 15) ÷ 16⌋ × 4` |
| 4 | `⌊(w + 7) ÷ 8⌋ × 4` |
| 8 | `⌊(w + 3) ÷ 4⌋ × 4` |
| 12 | `⌊(w + 1) ÷ 2⌋ × 4` |
| 16 | `⌊(w + 1) ÷ 2⌋ × 4` |
| 24 | `⌊(w × 3 + 11) ÷ 12⌋ × 12` |

Note the two irregularities, both confirmed by measurement over widths 1…12:

* **12 bpp occupies two whole bytes per pixel**, so its stride is the 16-bpp stride, not
  `⌈w × 12 / 8⌉` rounded up.
* **24 bpp rounds the row up to a whole multiple of four pixels (12 bytes)**, not to the usual
  4-byte boundary. Widths 1–4 all give 12 bytes, widths 5–8 give 24, widths 9–12 give 36.

Measured strides (width : bytes), which a reimplementation should reproduce exactly:

```
/1,/2      1:4  2:4  3:4  4:4  5:4  6:4  7:4  8:4  9:4  10:4  11:4  12:4
/4,/c4     1:4  2:4  3:4  4:4  5:4  6:4  7:4  8:4  9:8  10:8  11:8  12:8
/8,/c8     1:4  2:4  3:4  4:4  5:8  6:8  7:8  8:8  9:12 10:12 11:12 12:12
/c12,/c16  1:4  2:4  3:8  4:8  5:12 6:12 7:16 8:16 9:20 10:20 11:24 12:24
/c24       1:12 2:12 3:12 4:12 5:24 6:24 7:24 8:24 9:36 10:36 11:36 12:36
```

The pixel buffer is filled with `0xFF` before conversion and only the `w` real pixels of each row
are written. Therefore:

* **every padding byte at the end of a row is `0xFF`**, and
* for 1-, 2- and 4-bpp targets, **every padding bit inside the last partly-used byte is 1**.

The stored rows are **top-down**: row 0 of the stored data is the top row of the image.

Verified byte example — a 5×3 image at `/1` yields 12 bytes `EA FF FF FF | FC FF FF FF | E0 FF FF FF`.
Row 0 of the source is black, white, blue, green, red; their 1-bpp values are 0, 1, 0, 1, 0 packed
least-significant-bit first into `0xEA` = binary 1110 1010, with padding bits 1.

---

## 5. The `.mbm` file store layout (default, and with `/n`)

```
offset  size          contents
0       4             0x10000037
4       4             0x10000042
8       4             0x00000000
12      4             0x47396439        (fixed UID checksum for the three words above)
16      4             offset of the bitmap table
20      …             bitmap 1, bitmap 2, … laid end to end with no padding
T       4             number of bitmaps
T+4     4 × n         byte offset of each bitmap from the start of the file
```

* The bitmap-table offset written at offset 16 equals `20 + Σ bitmapSize`.
* The first bitmap always starts at offset 20. Each subsequent bitmap starts immediately after the
  previous one; **there is no alignment padding**, so a bitmap may start at an odd offset when the
  preceding one carries compressed data of odd length (verified: two 53-byte bitmaps give offsets
  20 and 73, table at 126, file length 138).
* Total file length is `20 + Σ bitmapSize + 4 + 4 × n`.
* `0x47396439` is a fixed constant; the tool writes it unconditionally and, when reading, rejects
  a file whose fourth word differs.

Each bitmap is a 40-byte header followed by its data:

| Offset in bitmap | Size | Field |
|---|---|---|
| 0 | 4 | total bitmap size = 40 + data length |
| 4 | 4 | header size, always 40 (`0x28`) |
| 8 | 4 | width in pixels |
| 12 | 4 | height in pixels |
| 16 | 4 | width in twips |
| 20 | 4 | height in twips |
| 24 | 4 | bits per pixel (1, 2, 4, 8, 12, 16 or 24) |
| 28 | 4 | colour flag: 0 for `/1 /2 /4 /8`, 1 for `/c4 /c8 /c12 /c16 /c24` |
| 32 | 4 | palette entry count — **always 0** in files this tool writes |
| 36 | 4 | compression type: 0 none, 1 bytewise RLE, 2 12-bit RLE, 3 16-bit RLE, 4 24-bit RLE |
| 40 | … | pixel data, uncompressed or compressed as indicated |

A bitmap whose pixel data is empty (width or height 0) is written as a 40-byte header with size
40 and no data; verified.

---

## 6. ROM image store layout (`/r` and `/s`)

```
offset  size          contents
0       4             0x10000041
4       4             number of bitmaps
8       4 × n         byte offset of each record from the start of the file
                      (first record starts at 4 × n + 8)
…       …             the records, each padded as described below
```

Each record is a 68-byte header followed by the pixel data:

| Offset in record | Size | Field |
|---|---|---|
| 0 | 4 | `0x10000040` |
| 4 | 4 | display-mode code (table below) |
| 8 | 4 | 0 |
| 12 | 4 | 0 |
| 16 | 4 | row stride in bytes (§4.6) |
| 20 | 40 | a verbatim copy of the 40-byte bitmap header of §5 |
| 60 | 4 | **never written** — remains `0xFFFFFFFF` because the record buffer is pre-filled with `0xFF` |
| 64 | 4 | `0x00000044` — the offset of the pixel data within the record |
| 68 | … | pixel data |

Display-mode codes:

| bpp | colour flag | code |
|---|---|---|
| 1 | — | 1 |
| 2 | — | 2 |
| 4 | 0 (grey) | 3 |
| 4 | 1 (colour) | 5 |
| 8 | 0 (grey) | 4 |
| 8 | 1 (colour) | 6 |
| 12 | — | 10 |
| 16 | — | 7 |
| 24 | — | 8 |

Record size is `(bitmapSize + 31) rounded down to a multiple of 4`, which is `68 + dataLength`
rounded up to a multiple of 4. The 0 to 3 trailing pad bytes are `0xFF`. The offset of each record
is the previous offset plus that rounded size.

If the palette-entry field of a bitmap were non-zero the record would be rejected with exit code
14, `Palettes not supported`; this never happens for bitmaps the tool builds.

`/r` writes every bitmap uncompressed. `/s` compresses every bitmap using the rules of §7; `/n` is
ignored when `/s` is given (verified: `/s` and `/s /n` produce identical bytes).

---

## 7. Compression

### 7.1 When it is applied

* File store, no `/n`: each bitmap is offered to the encoder for its depth; if the encoder gives
  up (see the size budget below), the bitmap is stored uncompressed with compression type 0.
* File store with `/n`: no compression; type 0.
* `/r`: no compression; type 0.
* `/s`: compression is attempted exactly as for the default file store, and may likewise fall
  back to type 0 for individual bitmaps.

The encoder chosen depends only on the stored bit depth:

| Stored bpp | Encoder | Type |
|---|---|---|
| 1, 2, 4, 8 | bytewise RLE | 1 |
| 12 | 12-bit RLE | 2 |
| 16 | 16-bit RLE | 3 |
| 24 | 24-bit RLE | 4 |

The input to the encoder is the **whole** pixel buffer including the `0xFF` row padding, so the
padding affects the compressed bytes.

### 7.2 Type 1 — bytewise RLE (1, 2, 4 and 8 bpp)

**Output grammar.** The stream is a sequence of blocks. Each block starts with one control byte,
interpreted as a signed 8-bit value:

* control 0 … 127 — a **run**: the single byte that follows is repeated `control + 1` times.
  A run therefore covers 1 to 128 bytes and costs 2 bytes.
* control −1 … −128 (i.e. byte values `0xFF` down to `0x80`) — a **literal**: the next
  `−control` bytes are copied verbatim. A literal covers 1 to 128 bytes and costs `1 + n` bytes.

**How the encoder splits the input.** Let `L` be the length of the pixel buffer.

1. Compute a guard length: `L ÷ 64`, truncated; if that is less than 5, use 4 instead. Call it
   `T`. The main pass only considers the first `L − T` bytes; the remainder is always emitted as
   a literal at the end. This is why an all-zero buffer never compresses down to the minimum
   possible size. Examples: `L` = 64 → `T` = 4; `L` = 256 → `T` = 4; `L` = 320 → `T` = 5;
   `L` = 512 → `T` = 8.
2. Compute the size budget: `⌊L ÷ 4⌋ + ⌊L ÷ 2⌋` (three quarters of the input, with the two
   truncations done separately).
3. Walk a cursor from the start of the buffer up to, but not including, position `L − T`. At each
   step let `c` be the byte under the cursor.
   * If the two bytes following the cursor both equal `c`, this is a run. Advance a scan pointer
     from cursor+3 while the byte it points at equals `c` **and** the pointer is still before
     `L − T`; the value test happens first, so the pointer stops at `L − T` at the latest. Emit a
     run of length `scan − cursor` (see the length coding below). Move the cursor to `scan`.
   * Otherwise this is a literal. Advance a scan pointer from the cursor while it is before
     `L − T` and it is *not* the case that both bytes following it equal the byte under it. Emit
     a literal covering `cursor` up to `scan`. Move the cursor to `scan`. The literal is always at
     least one byte long.
   * After each emitted block, if the number of output bytes written so far exceeds the budget,
     abandon compression entirely and store the bitmap uncompressed.
4. When the cursor reaches `L − T`, let `rem` = `L − cursor` (which is at least `T`). If
   `outputSoFar + rem` exceeds the budget, abandon compression. Otherwise emit a literal covering
   the remaining `rem` bytes. **Note the asymmetry:** this final check counts only the payload
   bytes, not the control byte(s) of that last literal, so the final compressed length may exceed
   the budget by one or more bytes. §14, example EX3 shows exactly this.

**Length coding for a run of `n` bytes** (`n ≥ 1`): if `n` > 128, emit `⌊(n − 1) ÷ 128⌋` copies of
the pair `7F` + value, then a final pair `(r − 1)` + value where `r = n − 128 × ⌊(n − 1) ÷ 128⌋`
and `r` is in 1…128.

**Length coding for a literal of `n` bytes** (`n ≥ 1`): if `n` > 128, emit `⌊(n − 1) ÷ 128⌋`
blocks of control byte `80` followed by 128 payload bytes, then a final control byte
`(256 − r)` followed by the remaining `r` payload bytes, with `r` as above.

Verified examples (all with `/8` and a greyscale source so that the stored bytes equal the source
grey levels):

| Input | `L` | `T` | Output |
|---|---|---|---|
| 8 zero bytes | 8 | 4 | `03 00 FC 00 00 00 00` |
| 64 zero bytes | 64 | 4 | `3B 00 FC 00 00 00 00` |
| 128 zero bytes | 128 | 4 | `7B 00 FC 00 00 00 00` |
| 132 zero bytes | 132 | 4 | `7F 00 FC 00 00 00 00` |
| 256 zero bytes | 256 | 4 | `7F 00 7B 00 FC 00 00 00 00` |
| 260 zero bytes | 260 | 4 | `7F 00 7F 00 FC 00 00 00 00` |
| 320 zero bytes | 320 | 5 | `7F 00 7F 00 3A 00 FB 00 00 00 00 00` |
| 512 zero bytes | 512 | 8 | `7F 00 7F 00 7F 00 77 00 F8` + eight `00` |
| 4 zero bytes | 4 | 4 | *rejected* (budget 3, 0 + 4 > 3) — stored uncompressed |
| 200 pseudo-random bytes | 200 | 4 | *rejected* — stored uncompressed |

### 7.3 Type 2 — 12-bit RLE

The pixel buffer is treated as a sequence of 16-bit little-endian words (two bytes per 12-bit
pixel, including padding words, which are `0xFFFF`). Each output word is 16 bits little-endian:

> output word = ((count − 1) × 4096) OR (value AND 0x0FFF)

where `count` is 1…16. A run of `n` identical words is emitted as `⌊(n − 1) ÷ 16⌋` words with
count nibble 15 (i.e. high nibble `F`) followed by one word with count `n − 16 × ⌊(n − 1) ÷ 16⌋`.

Run detection is simple and greedy: starting at a word, advance while the following words equal it
and the end of the buffer has not been reached.

**There is no size budget and no fallback** for 12-bit: a 12-bpp bitmap is always stored with
compression type 2. This is safe because each output word covers at least one input word, so the
stream never grows. Verified: a 24×1 image with 24 distinct colours produces 48 output bytes from
48 input bytes and is still marked type 2.

**Caution — the encoding is lossy for padding words.** The stored padding word `0xFFFF` has its
top nibble overwritten by the count, so it decodes back as `0x0FFF` rather than `0xFFFF`. A
reimplementation must reproduce the *encoder*'s bytes, not a round-trip-faithful encoding.

Verified example: a 13×2 image, left seven pixels black and right six white, at `/c12`. The
uncompressed data is, per row, seven words `0x0000`, six words `0x0FFF`, one padding word `0xFFFF`
(stride 28 bytes). The compressed output is `00 60 FF 5F FF 0F` repeated for the second row —
that is, run of 7 × `0x000` (count nibble 6), run of 6 × `0xFFF` (count nibble 5), run of 1 ×
`0xFFF` (count nibble 0).

### 7.4 Type 3 — 16-bit RLE

The pixel buffer is treated as `N = L ÷ 2` little-endian 16-bit words. Blocks:

* control 0 … 127 — a **run**: the two bytes that follow are one word repeated `control + 1`
  times. Cost 3 bytes.
* control −1 … −128 (`0xFF` down to `0x80`) — a **literal**: the next `−control` words (twice as
  many bytes) are copied verbatim. Cost `1 + 2n` bytes.

Budget: `(L × 7) ÷ 8`, truncated, where `L` is the buffer length in **bytes**.

Splitting:

1. While the cursor is before word `N − 1` (that is, at least two words remain):
   * If the word after the cursor equals the word at the cursor, this is a run. Advance a scan
     pointer from cursor+1: step it on by one, then test the word it points at — stop when that
     word differs, and stop in any case once the pointer has reached `N`. Emit a run covering
     `scan − cursor` words and move the cursor to `scan`. (The value test precedes the bound test,
     so the encoder reads one word past the end of the buffer at most; that read cannot change the
     output.)
   * Otherwise this is a literal. Advance a scan pointer from cursor+1 while it is before `N − 1`
     and the word after it differs from the word at it. Emit a literal covering `scan − cursor`
     words and move the cursor to `scan`.
   * After each block, if the output length exceeds the budget, abandon compression.
2. When the loop ends, if the cursor has not reached `N`, emit a literal covering the remaining
   `N − cursor` words. Then, if the output length exceeds the budget, abandon compression.

Length coding is the same shape as §7.2: runs longer than 128 words emit `7F` + word pairs;
literals longer than 128 words emit `80` + 256 payload bytes blocks; the final block uses
`(r − 1)` for a run or `(256 − r)` for a literal.

Verified example: a 13×2 image, left seven pixels black and right six white, at `/c16`. The
uncompressed data per row is seven words `0x0000` then seven words `0xFFFF` (stride 28). The
whole-buffer output is `06 00 00 | 06 FF FF | 06 00 00 | 06 FF FF`.

Verified rejection: a 24×1 image of 24 distinct colours at `/c16` gives 48 uncompressed bytes and
the compression is abandoned (type 0).

### 7.5 Type 4 — 24-bit RLE

The pixel buffer is treated as `L ÷ 3` three-byte pixels. Blocks:

* control 0 … 127 — a **run**: the three bytes that follow are one pixel repeated `control + 1`
  times. Cost 4 bytes.
* control −1 … −128 (`0xFF` down to `0x80`) — a **literal**: the next `−control` pixels (three
  times as many bytes) are copied verbatim. Cost `1 + 3n` bytes.

Budget: `(L × 7) ÷ 8`, truncated.

Splitting. Let `E` be the byte position `L − 3`.

1. While the cursor (a byte position, always a multiple of 3 from the start) is before `E`:
   * Compare the pixel at cursor+3 with the pixel at the cursor.
   * **If they are equal** — a run. Advance a scan pointer from cursor+3 in steps of 3: step it
     on, stop if it has reached `L`, otherwise stop when the pixel it points at differs. Emit a
     run of `(scan − cursor) ÷ 3` pixels and move the cursor to `scan`.
   * **If they differ** — a literal. Set the scan pointer to cursor+6 and remember the pixel at
     cursor+3 as the reference. If the scan pointer is already at or past `E`, the literal ends
     there. Otherwise set a look-ahead pointer to cursor+9 and repeat: if the pixel at the
     look-ahead pointer equals the reference, stop; otherwise make the pixel at scan+3 the new
     reference, advance both the scan and the look-ahead pointers by 3, and continue while the
     scan pointer is before `E`. Emit a literal of `(scan − cursor) ÷ 3` pixels and move the
     cursor to `scan`.

     **Quirk to reproduce exactly:** on the *first* iteration only, the reference is the pixel at
     cursor+3 while the look-ahead points at cursor+9 — a two-pixel gap. On every later iteration
     the reference and the look-ahead are three bytes apart, which is the intended "next two
     pixels are equal" test. This off-by-one affects where short literals end, and therefore the
     output bytes.
   * After each block, if the output length exceeds the budget, abandon compression.
2. When the loop ends, if the cursor has not reached `L`, emit a literal of `(L − cursor) ÷ 3`
   pixels. Then, if the output length exceeds the budget, abandon compression.

Length coding follows the same pattern: runs longer than 128 pixels emit `7F` + 3-byte pixel
groups; literals longer than 128 pixels emit `80` + 384 payload bytes blocks; the final block uses
`(r − 1)` for a run or `(256 − r)` for a literal.

Verified example: a 13×2 image, left seven pixels black and right six white, at `/c24`. Stride is
48 bytes, so each row holds 7 black pixels, 6 white pixels and 9 padding bytes (three more
"white" pixels as far as the encoder is concerned). The compressed output for the whole 96-byte
buffer is `06 00 00 00 | 08 FF FF FF | 06 00 00 00 | 08 FF FF FF`.

Verified rejection: a 24×1 image of 24 distinct colours at `/c24` gives 72 uncompressed bytes and
the compression is abandoned (type 0).

### 7.6 Degenerate case

A bitmap with zero pixel data (width or height 0) goes through the bytewise encoder, which emits
nothing and reports success. The bitmap is therefore stored with compression type **1** and no
data bytes. Verified: a 0×0 source produces a 68-byte file whose single bitmap has size 40, bpp 8,
compression 1.

---

## 8. Colour depth, greyscale and masks

### 8.1 Summary of the depth pipeline

Source BMP depth (1/4/8/16/24/32, with 2 and anything else silently producing white) → an
8-bit-per-channel RGB triple → the two substitutions of §3.3 → one of the nine target encodings of
§4 → row packing with `0xFF` padding → optional compression.

There is no dithering at any stage; every conversion is a per-pixel truncation or table lookup.
There is no gamma handling. There is no alpha handling: the fourth byte of a 32-bpp source is
dropped.

### 8.2 Masks

**`bmconv` has no notion of a mask.** There is no mask option, no mask flag in the bitmap header,
and no pairing of bitmaps in either container format. An icon mask is simply another bitmap in the
same `.mbm`, and the convention — mask immediately after its bitmap, usually built with `/1` or
`/8`, white meaning opaque — is enforced entirely by the consumer (the icon APIs, `mifconv`, and
the `AIF`/`MIF` tooling), not by `bmconv`.

Practical consequences for a reimplementation:

* A mask is produced by compiling a greyscale BMP with `/1` (1-bpp mask) or `/8` (8-bpp alpha-like
  mask). Nothing else distinguishes it.
* The colour flag in the header is 0 for such a bitmap, the same as any other greyscale bitmap.
* No inversion is applied by `bmconv`; the stored bits are exactly the quantised grey levels
  defined in §4.1. If a mask needs inverting, that must be done in the source BMP.
* Ordering in the `.mbm` is exactly the order of the source arguments; the tool never reorders.

---

## 9. The generated header file (`/h`)

### 9.1 Name derivation

Given a path, the *base name* is computed as follows:

1. Find the last `\` and the last `/`; take whichever is later; the name starts after it (or at
   the start of the string if neither is present).
2. Find the last `.`; if it is at or before the start position, the name runs to the end of the
   string; otherwise the name ends just before that dot.
3. Upper-case the first character and lower-case every other character. Characters that are not
   letters pass through unchanged.
4. If the name would be 256 characters or longer, the literal text `NameTooLong` is used instead.
   **Unverified** — a base name that long cannot be created on an ext4 filesystem; this rule comes
   from reading the tool, not from a run. Names of 200 characters were confirmed to pass through
   unchanged apart from the case folding.

Verified: `My_Header.MBG` → `My_header`; `MiXeD_Name-01.bmp` → `Mixed_name-01`; `x.bmp` → `X`;
`two.mbg` → `Two`; `m1.bmp` → `M1`.

### 9.2 File content

The enum is named after the **header file's** base name, not the `.mbm`'s. Each enumerator is
`EMbm` + the header base name + the source bitmap's base name, in source order, comma-separated,
with no comma after the last one. Every line ends with CRLF. The indentation is a single tab
character.

Exact layout, with `<H>` the header base name, `<F>` the header file name as it appears after the
last backslash (extension and original case preserved — note this step looks only for `\`, not
`/`), and `<B1>…<Bn>` the source base names:

```
// <F>
// Generated by BitmapCompiler
// Copyright (c) 1998-2001 Symbian Ltd.  All rights reserved.
//

enum TMbm<H>
→{
→EMbm<H><B1>,
→EMbm<H><B2>
→};
```

(`→` is a tab, every line ends CRLF, and there is a blank CRLF-only line after the `//` line.)

Verified byte example: `/hZ:\...\My_Header.MBG` with sources `MiXeD_Name-01.bmp` and `x.bmp`
produces

```
// My_Header.MBG<CR><LF>
// Generated by BitmapCompiler<CR><LF>
// Copyright (c) 1998-2001 Symbian Ltd.  All rights reserved.<CR><LF>
//<CR><LF>
<CR><LF>
enum TMbmMy_header<CR><LF>
<TAB>{<CR><LF>
<TAB>EMbmMy_headerMixed_name-01,<CR><LF>
<TAB>EMbmMy_headerX<CR><LF>
<TAB>};<CR><LF>
```

and the whole file is 151 bytes for the simpler single-bitmap case
`out1.mbg` / source `t8.bmp` (`enum TMbmOut1` with the single enumerator `EMbmOut1T8`).

If the header file cannot be created, the run fails with exit code 5,
`Bad destination file(s).` The header is written after the bitmaps have been converted but before
the container is written.

---

## 10. Failure modes and crashes

| Input | Result |
|---|---|
| Source file does not exist | exit 3, `File does not exist` |
| Source is not `BM`, or a short/oversized `bfSize`, or more than 256 palette entries | exit 4, `Bad source file(s).` |
| Source BMP with a non-zero compression field | exit 9, `Unknown source compression type.` |
| Destination path not creatable | exit 5, `Bad destination file(s).` |
| Header path not creatable | exit 5, `Bad destination file(s).` |
| `/r` and `/s` together | exit 8, `Too many arguments.` |
| Destination with no sources | exit 7, `Number of sources/targets mismatch.` |
| `/u` with more output names than bitmaps | exit 7 |
| Bad or unreadable palette file | exit 13 (short file) / exit 6 (missing file) |
| Bad depth option | exit 12, `Invalid bitmap mode specified.` |
| **Top-down BMP (negative height)** | **Crash.** An unhandled page fault on a write; no diagnostic and no output file. Reproduced with a 4×2 24-bpp top-down source at `/8`. A reimplementation should either support top-down sources properly or reject them with a clear message — but note that anything other than a crash is a behavioural difference. |
| 2-bpp BMP source | Silently produces an all-white bitmap, exit 0 |
| 16-bpp BMP source | Rejected (exit 4) unless "colours used" is forced to ≤ 256, in which case the colours produced are nonsense (§3.2) |
| Width or height 0 | Accepted; a 40-byte header with no data (compression type 1 if compression is enabled) |
| BMP palette shorter than the bit depth implies | **Unknown** — reads past the palette buffer; not characterised |

Other things that do **not** fail: 40 sources in one run (no argument limit was found); the same
source listed twice (two identical bitmaps, and two identically-named enumerators in the header).

---

## 11. Byte-for-byte reference examples

All examples below are complete files or complete bitmap payloads produced by the real tool.
They are suitable as golden test data.

### EX1 — 4×4, 8-bpp greyscale, uncompressed

Source: a 4×4 8-bpp BMP with a grey ramp palette (entry *i* = grey *i*) and pixel indices
0…15 in row order; 2835 pixels-per-metre on both axes.
Command: `/q /n <out.mbm> /8<in.bmp>`. Output file, 84 bytes:

```
37000010 42000010 00000000 39643947 4C000000
38000000 28000000 04000000 04000000 07000000 07000000 08000000 00000000 00000000 00000000
00010203 04050607 08090A0B 0C0D0E0F
01000000 14000000
```

### EX2 — 5×3 gradient at every depth, uncompressed

Source: a 5×3 24-bpp BMP, 2835 px/m, rows (as R,G,B):

* row 0: `000000  FFFFFF  0000FF  00FF00  FF0000`
* row 1: `302010  605040  908070  C0B0A0  F0E0D0`
* row 2: `030201  060504  090807  0C0B0A  0F0E0D`

Command: `/q /n <out.mbm> <OPT><in.bmp>`. All of these have pixel size 5×3, twips size 9×5, and
palette entries 0, compression 0.

| OPT | bpp | colour | stride | pixel data |
|---|---|---|---|---|
| `/1` | 1 | 0 | 4 | `EAFFFFFF FCFFFFFF E0FFFFFF` |
| `/2` | 2 | 0 | 4 | `8CFCFFFF A4FFFFFF 00FCFFFF` |
| `/4` | 4 | 0 | 4 | `F091F3FF 52B8FEFF 0000F0FF` |
| `/8` | 8 | 0 | 8 | `00FF1F9F3FFFFFFF 225282B2E2FFFFFF 0205080B0EFFFFFF` |
| `/c4` | 4 | 1 | 4 | `F079F5FF 10EEFFFF 0000F0FF` |
| `/c8` | 8 | 1 | 8 | `00FFDC1E05FFFFFF 07325D9193FFFFFF 0000000000FFFFFF` |
| `/c12` | 12 | 1 | 12 | `0000FF0F0F00F000000FFFFF 210354068709BA0CED0FFFFF 00000000000000000000FFFF` |
| `/c16` | 16 | 1 | 12 | `0000FFFF1F00E00700F8FFFF 023188620E9494C51AF7FFFF 00002000400841086108FFFF` |
| `/c24` | 24 | 1 | 24 | see the full listing below |

`/c24` pixel data in full (72 bytes, three rows of 24):

```
000000 FFFFFF FF0000 00FF00 0000FF FFFFFFFFFFFFFFFFFF
102030 405060 708090 A0B0C0 D0E0F0 FFFFFFFFFFFFFFFFFF
010203 040506 070809 0A0B0C 0D0E0F FFFFFFFFFFFFFFFFFF
```

### EX3 — bytewise RLE with a mixed run/literal split, and the budget quirk

Source: a 24×1 24-bpp BMP whose pixels are neutral greys with the values
`7 7 7 7 7 1 2 3 4 9 9 9 9 9 9 9 5 6 7 8 0 0 0 0`. Command: `/q <out.mbm> /8<in.bmp>`.
Uncompressed data would be those 24 bytes (stride 24). Output file, 87 bytes:

```
37000010 42000010 00000000 39643947 4F000000
3B000000 28000000 18000000 01000000 2F000000 01000000 08000000 00000000 00000000 01000000
04 07  FC 01 02 03 04  06 09  FC 05 06 07 08  FC 00 00 00 00
01000000 14000000
```

The compressed payload is 19 bytes while the budget is 18; this is the case described at the end
of §7.2 where the final literal's control byte is not counted.

### EX4 — two bitmaps in one file store, and the same bitmap as a ROM record

Sources: `a.bmp`, a 3×2 24-bpp image with top row red, green, blue and bottom row white, black,
`0x808080`; and `b.bmp`, a 2×2 1-bpp image with a two-entry palette of `000000` and `010101`.
Command: `/q <out.mbm> /c8<a.bmp> /1<b.bmp>`. Output file, 128 bytes:

```
37000010 42000010 00000000 39643947 74000000
30000000 28000000 03000000 02000000 05000000 03000000 08000000 01000000 00000000 00000000
05 1E DC FF   FF 00 70 FF
30000000 28000000 02000000 02000000 03000000 03000000 01000000 00000000 00000000 00000000
FC FF FF FF   FC FF FF FF
02000000 14000000 44000000
```

The `/c8` data confirms: red → 5, green → 30 (`1E`), blue → 220 (`DC`), white → 255, black → 0,
`0x808080` → substituted to `0x7F7F7F` → 112 (`70`); padding `FF`.

The same source at `/q /r <out.mbm> /c8<a.bmp>` gives an 88-byte ROM image:

```
41000010 01000000 0C000000
40000010 06000000 00000000 00000000 04000000
30000000 28000000 03000000 02000000 05000000 03000000 08000000 01000000 00000000 00000000
FFFFFFFF 44000000
05 1E DC FF   FF 00 70 FF
```

Note the `FFFFFFFF` at record offset 60 and the display-mode code 6 for 8-bpp colour.

### EX5 — a compressible bitmap in all three containers

Source: a 32×8 all-black 24-bpp image, 2835 px/m. Stored at `/c8` the uncompressed data is
256 zero bytes (stride 32), and the bytewise encoder produces `7F 00 7B 00 FC 00 00 00 00`
(9 bytes).

File store, `/q <out.mbm> /c8<in.bmp>`, 77 bytes:

```
37000010 42000010 00000000 39643947 45000000
31000000 28000000 20000000 08000000 3F000000 0F000000 08000000 01000000 00000000 01000000
7F 00 7B 00 FC 00 00 00 00
01000000 14000000
```

Compressed ROM image, `/q /s <out.mbm> /c8<in.bmp>`, 92 bytes (identical to `/q /s /n ...`):

```
41000010 01000000 0C000000
40000010 06000000 00000000 00000000 20000000
31000000 28000000 20000000 08000000 3F000000 0F000000 08000000 01000000 00000000 01000000
FFFFFFFF 44000000
7F 00 7B 00 FC 00 00 00 00
FF FF FF
```

The record size is `(49 + 31)` rounded down to a multiple of 4 = 80; the three trailing `FF`
bytes are the pad.

Plain ROM image, `/q /r ...`, is the same shape with compression type 0, bitmap size `0x128`
(296) and 256 zero data bytes.

### EX6 — the four compression types on the same picture

Source: a 13×2 24-bpp image, the leftmost 7 pixels of each row black and the remaining 6 white.

| OPT | uncompressed data | type | compressed data |
|---|---|---|---|
| `/8` | `00000000000000FF FFFFFFFFFFFFFFFF` ×2 (32 bytes, stride 16) | 1 | `06 00 08 FF 06 00 04 FF FC FF FF FF FF` (13 bytes) |
| `/c12` | per row: `0000`×7 `FF0F`×6 `FFFF` (stride 28, 56 bytes total) | 2 | `00 60 FF 5F FF 0F 00 60 FF 5F FF 0F` (12 bytes) |
| `/c16` | per row: `0000`×7 `FFFF`×7 (stride 28, 56 bytes total) | 3 | `06 00 00 06 FF FF 06 00 00 06 FF FF` (12 bytes) |
| `/c24` | per row: `000000`×7 then 27 bytes of `FF` (stride 48, 96 bytes total) | 4 | `06 00 00 00 08 FF FF FF 06 00 00 00 08 FF FF FF` (16 bytes) |

### EX7 — `/u` round trip

`/u` on the EX4 file store with two output names produces two 24-bpp BMPs:

```
u1.bmp (78 bytes), stride 12:
424D 4E000000 0000 0000 36000000
28000000 03000000 02000000 0100 1800 00000000 00000000 00000000 00000000 00000000 00000000
FFFFFF 000000 777777 FFFFFF
0000FF 00FF00 FF0000 FFFFFF

u2.bmp (70 bytes), stride 8:
424D 46000000 0000 0000 36000000
28000000 02000000 02000000 0100 1800 00000000 00000000 00000000 00000000 00000000 00000000
000000 000000 FFFF
000000 000000 FFFF
```

The trailing `FF` bytes of each row are the padding (three bytes in `u1`, two in `u2`). Rows are
written bottom-up, so the first row of pixel data is the bottom row of the image: in `u1` that is
white, black and `0x777777` — the last of which shows that the `/c8` index 112 round-trips back to
the palette colour `0x777777`, not to the original `0x808080`.

---

## 12. Anything else that changes output bytes

A checklist for the implementer, all of it verified unless noted:

1. The `0x808080` → `0x7F7F7F` and `0xC0C0C0` → `0xBBBBBB` substitutions (§3.3).
2. Row padding bytes are `0xFF`, padding bits are 1 (§4.6), and padding participates in
   compression.
3. Rows are stored top-down while the BMP is read bottom-up.
4. The 24-bpp stride rounds to whole groups of four pixels, and the 12-bpp stride is the 16-bpp
   stride (§4.6).
5. The twips conversion is the tool's own, off by a factor of ten from the physical one, and
   truncates twice (§4.2).
6. The bytewise encoder's guard tail (`max(4, L ÷ 64)`) and the 3/4 budget with its off-by-one
   final check (§7.2).
7. The 24-bit encoder's first-iteration look-ahead gap (§7.5).
8. 12-bpp bitmaps are always marked compressed and their padding words lose their top nibble
   (§7.3).
9. The ROM record's never-written word at offset 60 (`0xFFFFFFFF`) and the `0xFF` tail padding
   (§6).
10. Bitmaps are packed into the file store with no alignment padding (§5).
11. `/s` ignores `/n`; `/r` never compresses.
12. `/p` affects only `/c8` and is not recorded in the output file.
13. The header file's enum name comes from the `/h` file name, not the `.mbm` name; the first
    comment line preserves the original case and extension and splits on `\` only (§9).
14. CRLF line endings in the header file.
15. The source BMP's declared file size, not its real length, determines how much pixel data is
    read (§3.1).

---

## 13. Items marked unknown

* The behaviour for source images wide enough to overflow the twips multiplication (widths above
  roughly 1491 pixels with the usual 2835 px/m). Not tested.
* The exact output when a source BMP declares fewer palette entries than its bit depth requires
  and a pixel index exceeds the palette. The tool reads past the allocated palette; the resulting
  bytes were not characterised.
* The `NameTooLong` substitution in the header file (base names of 256 characters or more) could
  not be exercised because the host filesystem caps names at 255 bytes. The rule is stated from
  reading the tool only.
* The exact stdout interleaving of the `Header file generation is not permitted with /m` warning
  relative to the banner (it appeared before the banner in the captured output, which is probably
  a stream-flush artefact).
* Behaviour with a BMP whose info header is 108 or 124 bytes (V4/V5). Not tested; from the read
  sequence it would be misparsed rather than rejected cleanly.
* Whether any SDK build other than version 112 differs. Everything here describes version 112 as
  shipped in `S60_3rd_FP2`.
