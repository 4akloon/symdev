# Binary SVG (`.svgb`) and the Multi Icon File (`.mif`/`.mbg`)

Written 2026-09-20 by a separate agent from disassembly of
`epoc32/tools/svgtbinencode.exe` and `epoc32/tools/mifconv.exe` (S60 3rd Edition
FP2 SDK) plus black-box runs of both tools under Wine, approved by the
repository owner for the purpose of a clean-room native reimplementation. The
implementer who works from this document did not see the disassembly and must
not: everything needed is stated here as prose, tables and verified byte
examples. Tool identity for the record: `MifConv version 1.11 build (49, SVG
stand-alone)`; the SVG encoder prints no version banner.

Every byte sequence quoted below was produced by running the real tools on the
input shown, on this host. Anything that could not be established is marked
**unknown** rather than guessed.

---

## 1. Confidence map

| Area | Status |
| --- | --- |
| `.mif` container, `.mbg` header, depth codes | fully pinned; a byte-exact rebuild of a one-icon and a two-icon file was verified |
| `.svgb` header, tree serialisation, numbers, strings, colours | fully pinned |
| `.svgb` for `<svg>`+`viewBox`+`<rect>`(+`rx`/`ry`)+`<circle>`+`fill` (the subset symdev's icon template uses) | fully pinned; a byte-exact rebuild of the shipped template was verified |
| Element token table | pinned except three token values (see §4.8) |
| Attribute id table | pinned for ~80 attributes (§4.9); a few ids unassigned |
| Paths, polylines, transforms, gradients, text | pinned for the cases listed; not exhaustively |
| Animation elements, fonts, `<image>` payloads | only token ids pinned, value layouts largely unexplored |

---

## 2. `svgtbinencode` command line

Invocation: `svgtbinencode.exe [-d] [-v <n>] [-h] <file.svg>`.

The tool prints its own help when given `-h` or `-help` (it also prints the help
after the "invalid/no file name" error when no file is given). Reproduced as the
tool prints it:

```
-------Encoder Help--------
Parameters:
-d for debug mode
-v <number> for encoding version
-v 1 original encoding for 3.0 and 3.1 (BGR/float)
-v 2 encoding (BGR/fixed point)
-v 3 optimized encoding for 3.1 only (RGB/fixed pt)
-v 4 encoding (RGB/float)
-h for help
```

* The version number is a **separate argument** after `-v` (`-v 3 file.svg`).
  `mifconv` passes it that way. An attached form was not tested.
* Default when `-v` is absent: **3**.
* `-d` prints `Encoder Debugging On...`, then ` Encoding complete !!!` and
  `Press Any Key` at the end. Without `-d` a successful run prints nothing.
* Exactly one input file; a second file name was not tested.
* Output is written next to the **input** file, not in the current directory:
  the input path with `.svg` replaced by `.svgb`. Verified: encoding
  `sub/z.svg` wrote `sub/z.svgb`.
* Any pre-existing `.svgb` is overwritten.

Meaning of the version number (confirmed by byte diffs, §4.2 and §4.6):

| `-v` | number encoding | colour word |
| --- | --- | --- |
| 1 | IEEE-754 single, little-endian | `0x00BBGGRR` |
| 2 | 16.16 signed fixed point, little-endian | `0x00BBGGRR` |
| 3 | 16.16 signed fixed point, little-endian | `0x00RRGGBB` |
| 4 | IEEE-754 single, little-endian | `0x00RRGGBB` |

Exit codes observed:

| Situation | stdout | exit |
| --- | --- | --- |
| success | nothing (or the debug lines with `-d`) | 0 |
| no file name | `ERROR: Invalid/No file name provided.` + help | 1 |
| `-h` | error line + help | 1 |
| input does not exist | `ERROR: SVG file not exist for Encoding : <name>` | 1 |
| input name does not end in `.svg` | `ERROR:  Input parameter is not valid svg file <name>` | 1 |
| `-v` present but the number is missing/garbled | `ERROR: Invalid version number` | 1 (no output file) |
| input is not well-formed XML | **nothing** | **0**, and a 5-byte output file is written (§6.2) |
| text content longer than 255 characters | nothing | killed by a segmentation fault; a truncated output file is left behind |

The silent-success-on-broken-XML case is the dangerous one: a native
reimplementation should reject malformed XML loudly, and any consumer of the
SDK tool must check the output size, not just the exit code.

---

## 3. `mifconv` command line

Usage exactly as the tool prints it:

```
MIFCONV miffile.MIF
        [/Hheaderfile.MBG]
        [/E]
        [/Ppalettefile]
        [/Ttemppath]
        [/Bbmconvpath]
        [/Ssvgencodepath]
        [/Vsvgtversion]
        [/Fparametername.txt]
        [/A] [/OPT] iconsource1.EXT [ ... [/A] [/OPT] iconsourceN.EXT]
```

* `/H<file>.mbg` — write the C++ enum header (§7). Optional; without it only the
  `.mif` is produced.
* `/E` — take source icons only with the extension given. By default `mifconv`
  prefers a `.svg` sibling over the extension actually named: naming `a.txt`
  while `a.svg` exists silently builds from `a.svg` and exits 0.
* `/P<file>` — palette file, forwarded to `bmconv` for bitmap icons.
* `/T<dir>` — temporary directory. Required on this host: the built-in default
  `\epoc32\BUILD\s60\icons\temp\` does not exist and the run stops with
  `ERROR: Changing temporary working directory failed!`.
* `/B<dir>` — directory holding `bmconv.exe`.
* `/S<dir>` — directory holding `svgtbinencode.exe`. Required for SVG sources;
  without it the run stops with
  `ERROR: Binary converter 'SVGTBINENCODE.exe' not found`.
* `/V<n>` — the `-v` value handed to the SVG encoder; 1–4, default 3. `/V5`
  prints the usage text.
* `/F<file>` — parameter file; its contents are the same options and sources,
  separated by spaces or newlines. Verified working.
* `/A` — set the animated flag on the **next** icon source.
* `/OPT` — `DEPTH[,MASK]` for the next icon source; `DEPTH` is one of
  `/1 /2 /4 /8 /c4 /c8 /c12 /c16 /c24 /c32` and `MASK` is `1` or `8`.
* `EXT` may be `SVG` or `BMP`.

Exit codes: 0 on success, 1 for too few arguments, a missing source, a missing
`/S`, a missing `/T`, or an invalid depth. All error lines are prefixed
`ERROR: ` and most are followed by the full usage text.

### 3.1 Traps that change or destroy the output

These were all reproduced on this host and matter to anyone still shelling out
to the SDK tool.

1. **`/V` must not come directly after `/T`.** `… /Tt /V1 /c32,8 a.svg` fails
   with `ERROR: Invalid version number` followed by
   `ERROR: Unable to open file for reading! t\<tmp>\a.svgb`, and no `.mif` is
   written. Every other order tested works: `/V1 /S… /T…`, `/S… /V1 /T…`,
   `/T… /S… /V1`, `/S… /T… /OPT /V1`.
2. **Source paths must use backslashes.** `sub/nested.svg` fails with
   `ERROR: File copy failed: t\<tmp>\sub/nested.svg`; `sub\nested.svg` works.
3. **`mifconv` mangles the source path into the temporary file name.** It
   creates a uniquely named subdirectory under `/T` (names seen: `s8s.tmp`,
   `s10.tmp`, `smk.tmp`) and copies the source into it under a name in which
   the drive colon and every separator become `_`. A relative source `icon.svg`
   becomes `t\s8s.tmp\icon.svgb`; an absolute source
   `Z:\tmp\…\mif\a.svg` becomes
   `t\s6k.tmp\Z__tmp_claude-1000_svgb-spec-work_mif_a.svgb`. When that composed
   name gets long the encoder writes nothing, `mifconv` still exits 0, and the
   `.mif` contains an icon of length 0. Run with a short relative source path.
4. **The icon name in the `.mbg` is not sanitised.** `My.Icon.svg` yields the
   enumerator `EMbmNMy.icon`, which is not a legal C++ identifier (§7).

---

## 4. The `.svgb` format

### 4.1 File header

Four bytes, then the document. There is **no** string table, number table,
length field or index anywhere in the file: the document is a single forward
stream.

| Offset | Size | Content |
| --- | --- | --- |
| 0 | 1 | `0xCB + version`, i.e. `0xCC`, `0xCD`, `0xCE`, `0xCF` for `-v 1`…`-v 4` |
| 1 | 3 | `56 FA 03` |

Equivalently the four bytes are a little-endian word `0x03FA56CC + (version-1)`.
Verified by encoding the same one-rectangle document at each version: the files
differ in byte 0 (`cc`/`cd`/`ce`/`cf`) and in the number and colour encodings,
nowhere else.

The smallest well-formed document, `<svg xmlns="http://www.w3.org/2000/svg"/>`,
gives exactly 9 bytes at `-v 3`:

```
ce 56 fa 03 00 e8 03 fe ff
```

(header, `<svg>` token, end-of-attributes, end-of-element, end-of-document — 9
bytes). A document with no usable root at all gives `ce 56 fa 03 ff`.

### 4.2 Document tree

The tree is serialised depth-first with these markers:

| Marker | Meaning |
| --- | --- |
| one byte, value ≤ `0x30` | start of an element; the value is the element token (§4.8) |
| `E8 03` (little-endian 1000) | end of the element's attribute list; children follow |
| `FD` | start of the element's character data (only seen on `<text>`) |
| `FE` | end of an element |
| `FF` | end of the document; the last byte of the file |

So an element is: token, zero or more attribute records, `E8 03`, then children
(each itself an element or a `FD` text block), then `FE`. The root element's
`FE` is followed by the single `FF`.

`<svg xmlns="…"><g><g><rect/></g><rect/></g></svg>` at `-v 3`:

```
ce 56 fa 03  00  e8 03  0b e8 03  0b e8 03  21 e8 03 fe  fe  21 e8 03 fe  fe  fe ff
             svg  (end)  g  (end)  g  (end)  rect        /g  rect         /g   /svg EOF
```

Attributes are written in **document order**, not sorted and not normalised.
`<rect fill="#ff0000" height="40"/>` and `<rect height="40" fill="#ff0000"/>`
produce the same bytes in swapped order:

```
21  00 00 00 00 00 ff 00   1b 00 00 00 28 00   e8 03 fe
21  1b 00 00 00 28 00   00 00 00 00 00 ff 00   e8 03 fe
```

Unknown elements, unknown attributes, XML comments, the XML declaration, the
`xmlns`/`xmlns:xlink` declarations and `<?…?>` processing instructions produce
no bytes at all. An unknown element's *children* are dropped with it (verified
with `<metadata/>`, `<script/>`, `<tspan/>`, `<video/>`, `<foreignObject/>`,
`<pattern/>`, `<clipPath/>`, `<mask/>`, `<symbol/>`, `<filter/>`, `<marker/>`,
`<textPath/>`, `<tref/>`, `<tbreak/>`, `<textArea/>`, `<handler/>`,
`<listener/>`, `<prefetch/>`, `<cursor/>`, `<color-profile/>`).

### 4.3 Attribute records

An attribute record is a **16-bit little-endian attribute id** followed by the
value, whose layout depends on the attribute (§4.9). There is no length field
and no type byte in the general case; the reader is expected to know the layout
from the id. The attribute list ends when the id `0x03E8` (1000) is read, which
is why no real attribute may use that id.

### 4.4 Numbers

A number is 4 bytes, little-endian:

* versions 2 and 3: signed 16.16 fixed point, i.e. the value times 65536;
* versions 1 and 4: IEEE-754 single precision.

Conversion rules, all verified on `<rect x="…">` at `-v 3` (the value bytes
shown are the four that follow the id `31 00`):

| Input | Bytes | Value | Note |
| --- | --- | --- | --- |
| `10` | `00 00 0a 00` | 10.0 | |
| `1.5` | `00 80 01 00` | 1.5 | |
| `0.5` | `00 80 00 00` | 0.5 | |
| `0.1` | `99 19 00 00` | 6553/65536 | 0.1 × 65536 = 6553.6 → **truncated toward zero** |
| `-0.1` | `67 e6 ff ff` | −6553/65536 | also truncated toward zero, not floored |
| `0.999999` | `ff ff 00 00` | 65535/65536 | truncated |
| `0.00002` | `01 00 00 00` | 1/65536 | |
| `0.000014` | `00 00 00 00` | 0 | |
| `1234.5` | `00 80 d2 04` | 1234.5 | |
| `32765` | `00 00 fd 7f` | 32765.0 | largest accepted |

The parse is **single precision**: `8191.99998` encodes as exactly 8192.0
(`00 00 00 20`) because that is the nearest `float`, and `32765.00001` is
accepted because it rounds to 32765.0.

**Range check.** A number whose magnitude exceeds 32765.0 makes the encoder drop
the whole attribute. `<rect x="32765"/>` encodes; `<rect x="32765.5"/>`,
`<rect x="32766"/>`, `<rect x="-32766"/>`, `<rect x="1e6"/>` all produce
`21 e8 03` — the element with no attributes. The constant 32765.0 is present in
the binary as a single-precision literal, so the test is `|v| > 32765.0f`.

At `-v 1`/`-v 4` the same values are IEEE floats: `x="10"` gives
`00 00 20 41`, `width="30"` gives `00 00 f0 41`, `width="88"` gives
`00 00 b0 42`.

**Negative values.** Negative numbers are stored as negative fixed point
(`rx="-1"` → `00 00 ff ff`, `stroke-width="-1"` → `00 00 ff ff`), except where
SVG forbids them: `<rect width="-1">` and `<svg width="-5">` drop the attribute
entirely.

**Units.** A CSS unit suffix is parsed and then **ignored**: `1in`, `1cm`,
`12pt`, `3pc`, `2mm`, `10em`, `10ex` encode as the bare number 1, 1, 12, 3, 2,
10, 10. Only `%` is preserved, and only on the attributes that carry a unit byte
(§4.5).

### 4.5 Lengths with a unit byte

`width` and `height` **on the `<svg>` element only** are written as one extra
leading byte followed by the number: `00` for a plain user-unit value, `01` for
a percentage. The same ids on `<rect>` and `<image>` have no such byte.

```
<svg width="100">    →  1a 00  00  00 00 64 00
<svg width="50%">    →  1a 00  01  00 00 32 00
<rect width="3">     →  1a 00      00 00 03 00
<image width="4">    →  1a 00      00 00 04 00
```

This applies to a nested `<svg>` too, so it is a property of the element, not of
being the document root.

Other unit bytes than `00` and `01` are **unknown**.

### 4.6 Colours and paint

A colour is a 4-byte little-endian word. At `-v 3` and `-v 4` it is
`0x00RRGGBB`; at `-v 1` and `-v 2` it is `0x00BBGGRR`. Input
`fill="#123456"`:

| version | bytes |
| --- | --- |
| 1, 2 | `12 34 56 00` |
| 3, 4 | `56 34 12 00` |

The top byte carries a paint keyword instead of a colour:

| Value | Word | Bytes (`-v 3`) |
| --- | --- | --- |
| a colour | `0x00RRGGBB` | e.g. `#ff0000` → `00 00 ff 00` |
| `none` | `0x01FFFFFF` | `ff ff ff 01` |
| `currentColor` | `0x02FFFFFF` | `ff ff ff 02` |

Accepted colour syntaxes: `#rrggbb` (`#aabbcc` → `cc bb aa 00`),
`rgb(r,g,b)` (`rgb(1,2,3)` → `03 02 01 00`), `rgb(p%,p%,p%)`
(`rgb(50%,0%,0%)` → `00 00 7f 00`, so 50 % becomes 127) and the SVG colour
keyword list (`red` → `00 00 ff 00`). An unrecognised keyword or a malformed hex
value silently becomes black: `fill="bogus"` and `fill="#GG0000"` both give
`00 00 00 00`.

**The three-digit `#rgb` form is expanded wrongly** and a byte-identical
reimplementation has to copy the bug: each channel becomes
`(digit << 4) | 0x0F`, not `digit × 0x11`. Verified at `-v 3`:

| Input | Bytes | Word | Correct SVG value |
| --- | --- | --- | --- |
| `#abc` | `cf bf af 00` | `0x00AFBFCF` | `0x00AABBCC` |
| `#f0a` | `af 0f ff 00` | `0x00FF0FAF` | `0x00FF00AA` |
| `#000` | `0f 0f 0f 00` | `0x000F0F0F` | `0x00000000` |
| `#fff` | `ff ff ff 00` | `0x00FFFFFF` | `0x00FFFFFF` (agrees by luck) |

`fill` — and only `fill` — has an extra **leading flag byte** before the value,
because it is the one paint that may be a gradient reference:

| Input | Bytes after the id `00 00` |
| --- | --- |
| `fill="#010203"` | `00` `03 02 01 00` |
| `fill="none"` | `00` `ff ff ff 01` |
| `fill="url(#g)"` | `01` `02 67 00` — flag 1, then a string (§4.7) holding `g` without the `#` |

`stroke`, `color` and `stop-color` have **no** flag byte and are always the bare
4-byte word:

```
<rect stroke="#010203"/>      →  21  01 00  03 02 01 00  e8 03 fe
<rect color="#010203"/>       →  21  0f 00  03 02 01 00  e8 03 fe
```

`stroke="url(#g)"` is **not** supported: it encodes as `01 00 00 00 00 00`, i.e.
black. `stroke="inherit"` encodes as `none` (`ff ff ff 01`) while
`color="inherit"` and `fill="inherit"` encode as black (`00 00 00 00`).

Opacities are ordinary numbers clamped to 0…1: `opacity="0.5"` →
`18 00 00 80 00 00`, `opacity="2"` → `00 00 01 00`, `opacity="-1"` →
`00 00 00 00`.

### 4.7 Strings

A string value is **one length byte giving the length in bytes** followed by
that many bytes of UTF-16LE. There is no terminator.

```
<svg baseProfile="tiny"/>  →  59 00  08  74 00 69 00 6e 00 79 00
<rect id="abc"/>           →  5c 00  06  61 00 62 00 63 00
```

XML entity references are resolved before encoding (`a&amp;b` → `a&b`) and
leading, trailing and repeated whitespace in character data is collapsed
(`"  a  b  "` → `"a b"`). CDATA sections become ordinary character data.

**The length byte is only 8 bits and the encoder does not check it.** A value
of *n* characters writes `(2n) mod 256` as the length and then exactly that many
bytes — the rest of the string is thrown away. Verified: a 126-character `id`
writes length `fc` (252) and 252 bytes; 127 characters write `fe`; **128
characters write `00` and no characters at all**; 200 characters write `90`
(144) and 144 bytes. A native encoder must reject strings longer than 127
characters rather than reproduce this.

Character data has the same limit and a worse failure: 255 characters encode
(length `fe`), 256 characters encode as an empty text block, and 300 characters
crash the tool with a segmentation fault after writing a truncated file.

A *list of strings* (`requiredFeatures`, `requiredExtensions`,
`systemLanguage`) is a one-byte item count followed by that many strings:

```
<rect requiredFeatures="a b"/>  →  61 00  02  02 61 00  02 62 00
<rect systemLanguage="en"/>     →  62 00  01  04 65 00 6e 00
```

### 4.8 Element tokens

One byte, written where an element starts.

| Token | Element | Token | Element |
| --- | --- | --- | --- |
| `0x00` | `svg` | `0x19` | `text` |
| `0x01` | `altGlyph` | `0x1a` | `use` |
| `0x02` | `altGlyphDef` | `0x1b` | `circle` |
| `0x03` | `defs` | `0x1c` | `ellipse` |
| `0x04` | `desc` | `0x1d` | `line` |
| `0x05` | **unknown** | `0x1e` | `path` |
| `0x06` | **unknown** | `0x1f` | `polygon` |
| `0x07` | `title` | `0x20` | `polyline` |
| `0x08` | `font-face-name` | `0x21` | `rect` |
| `0x09` | `font-face-src` | `0x22` | `animate` |
| `0x0a` | `font-face-uri` | `0x23` | `animateColor` |
| `0x0b` | `g` | `0x24` | `animateMotion` |
| `0x0c` | `glyphRef` | `0x25` | `animateTransform` |
| `0x0d` | `vkern` | `0x26` | `set` |
| `0x0e` | **unknown** | `0x27` | `mpath` |
| `0x0f` | `switch` | `0x28` | `linearGradient` |
| `0x10` | `view` | `0x29` | `radialGradient` |
| `0x11` | `hkern` | `0x2a` | `stop` |
| `0x12` | `a` | `0x2b` | **unknown** |
| `0x13` | `font` | `0x2c` | **unknown** |
| `0x14` | `font-face` | `0x2d` | `discard` |
| `0x15` | `glyph` | `0x2e` | `solidColor` |
| `0x16` | `image` | `0x2f` | `animation` |
| `0x17` | `missing-glyph` | `0x30` | `audio` |
| `0x18` | `style` | | |

`<animateMotion/>` with no attributes emits a 7-byte attribute record of its own
accord (`7a 00 00 00 00 00 94`); that default is **unknown** and was not chased.

### 4.9 Attribute ids

Ids are global, not per element: `width` is `0x001a` on `<svg>`, `<rect>` and
`<image>` alike; `x` is `0x0031` on `<rect>`, `<text>` and `<use>`.

"Value" gives the layout that follows the 16-bit id. *number* = one 4-byte
number (§4.4); *colour* = one 4-byte colour word (§4.6); *string* = length byte
+ UTF-16LE (§4.7); *enum32* = a 4-byte little-endian integer; *enum8* = a single
byte.

| Id | Attribute | Value |
| --- | --- | --- |
| `0x0000` | `fill`, and `solid-color` on `<solidColor>` | flag byte + colour, or flag `01` + string (§4.6) |
| `0x0001` | `stroke` | colour |
| `0x0002` | `stroke-width` | number |
| `0x0003` | `visibility` | enum32: `visible`/`inherit`/unrecognised → 0, `hidden` → 1, `collapse` → 3 |
| `0x0004` | `font-family` | string |
| `0x0005` | `font-size` | number |
| `0x0006` | `font-style` | enum32: `normal` 0, `italic` 1, `oblique` 2 |
| `0x0007` | `font-weight` | enum32: `normal` 0, `bold` 1, `bolder` 2, `lighter` 3, `100`…`900` → 4…12 |
| `0x0008` | `stroke-dasharray` | one-byte item count + that many numbers |
| `0x0009` | `display` | enum32: `inline`/`block`/`inherit` → 0, `none` → 16 |
| `0x000a` | `fill-rule` | string (the keyword is stored verbatim, not as an enum) |
| `0x000b` | `stroke-linecap` | string |
| `0x000c` | `stroke-linejoin` | string |
| `0x000d` | `stroke-dashoffset` | number |
| `0x000e` | `stroke-miterlimit` | number |
| `0x000f` | `color` | colour |
| `0x0010` | `text-anchor` | enum32 (mapping **unknown**) |
| `0x0011` | `text-decoration` | enum32 (mapping **unknown**) |
| `0x0016` | `fill-opacity`, and `solid-opacity` on `<solidColor>` | number, clamped 0…1 |
| `0x0017` | `stroke-opacity` | number, clamped 0…1 |
| `0x0018` | `opacity` | number, clamped 0…1 |
| `0x0019` | `textLength` | number |
| `0x001a` | `width` | number; unit byte first on `<svg>` (§4.5) |
| `0x001b` | `height` | number; unit byte first on `<svg>` |
| `0x001c` | `r` | number |
| `0x001d` | `rx` | number |
| `0x001e` | `ry` | number |
| `0x001f` | `horiz-adv-x` | number |
| `0x0022` | `ascent` | number |
| `0x0023` | `descent` | number |
| `0x002b` | `units-per-em` | number |
| `0x002e` | `cx` | number |
| `0x002f` | `cy` | number |
| `0x0030` | `y` | number; on `<text>` a one-byte count + that many numbers |
| `0x0031` | `x` | number; on `<text>` a one-byte count + that many numbers |
| `0x0032` | `y1` | number |
| `0x0033` | `y2` | number |
| `0x0034` | `x1` | number |
| `0x0035` | `x2` | number |
| `0x0036` | `k` | number |
| `0x0042` | `transform` | 6 numbers + a 4-byte kind word (§4.11) |
| `0x0047` | `from` | **unknown** (one byte seen for a numeric value) |
| `0x0048` | `to` | **unknown** |
| `0x0049` | `by` | **unknown** |
| `0x004a` | `attributeName` | **unknown** (3 bytes seen) |
| `0x004c` | `version` | number — `version="1.1"` → `99 19 01 00` (1.1 in 16.16) |
| `0x004e` | `points` | path payload (§4.10) |
| `0x004f` | `d` | path payload (§4.10) |
| `0x0051` | `stop-color` | colour (no flag byte) |
| `0x0052` | `fx` | number |
| `0x0053` | `fy` | number |
| `0x0054` | `offset` | number |
| `0x0055` | `spreadMethod` | enum8: `pad`/unrecognised 0, `reflect` 1, `repeat` 2 |
| `0x0056` | `gradientUnits` | enum8: `userSpaceOnUse` 0, `objectBoundingBox` 1 |
| `0x0057` | `stop-opacity` | number |
| `0x0058` | `viewBox` | 4 numbers, min-x min-y width height, no count |
| `0x0059` | `baseProfile` | string |
| `0x005a` | `zoomAndPan` | enum8; `disable` → `00`. Only accepted on `<svg>` |
| `0x005b` | `preserveAspectRatio` | 2 bytes; **only the value `none` is emitted**, as `00 02`. Every other value, including `xMidYMid meet`, `xMinYMin meet` and `xMaxYMax slice`, is dropped |
| `0x005c` | `id` | string |
| `0x005d` | `xml:base` | string |
| `0x005e` | `xml:lang` | string |
| `0x005f` | `xml:space` | string |
| `0x0060` | `requiredExtensions` | count byte + strings |
| `0x0061` | `requiredFeatures` | count byte + strings |
| `0x0062` | `systemLanguage` | count byte + strings |
| `0x006d` | `xlink:href` | string — but see the note below |
| `0x006e` | `begin` | variable-length record, partly **unexplained** (below) |
| `0x006f` | `dur` | 4-byte little-endian **milliseconds**, then one `00` byte: `1s` → `e8 03 00 00 00`, `2` → `d0 07 00 00 00`, `indefinite` → `ff ff ff 7f 00` |
| `0x0070` | `repeatCount` | number |
| `0x0071` | `repeatDur` | 4-byte little-endian milliseconds: `1s` → `e8 03 00 00`, `2` → `d0 07 00 00`, `indefinite` → `ff ff ff 7f` |
| `0x0072` | `end` | variable-length record, partly **unexplained** (below) |
| `0x0073` | `restart` | **unknown** |
| `0x0076` | `keySplines` | **unknown** |
| `0x0078` | `calcMode` | **unknown** |
| `0x0081` | `syncBehavior` | **unknown** |
| `0x0082` | `syncTolerance` | **unknown** |
| `0x0083` | `syncMaster` | **unknown** |
| `0x0085` | `volume` | number |
| `0x0086` | `audio-level` | number |
| `0x00a3` | `values` | **unknown** |
| `0x03E8` | — | reserved: the end-of-attribute-list marker |

Ids not listed are **unknown**. Attributes not in the table produce no bytes:
`class`, `style` (see below), `pathLength`, `color-rendering`,
`color-interpolation`, `letter-spacing`, `word-spacing`, `rotate`, `type`,
`lang`, `unicode`, `glyph-name`, `u1`, `u2`, `g1`, `g2`, `end`, `additive`,
`accumulate`, `keyTimes`, `focusable`, `initialVisibility`,
`externalResourcesRequired`, `target`, `media`, `snapshotTime`,
`playbackOrder`, `timelineBegin`, `editable`, `gradientTransform`,
`nav-*`, and any attribute the encoder does not recognise.

Two oddities that must be reproduced for byte identity:

* `style` is **parsed** and turned into the corresponding property records:
  `<rect style="fill:red"/>` encodes exactly like `<rect fill="red"/>`
  (`21 00 00 00 00 00 ff 00 e8 03 fe`).
* `xlink:href` is written **twice** on `<use>` and once on `<a>` and `<image>`:

  ```
  <use xlink:href="#a"/>    →  1a  6d 00 04 23 00 61 00  04 23 00 61 00  e8 03 fe
  <a xlink:href="u">…       →  12  6d 00 02 75 00  e8 03 …
  <image xlink:href="p.png" width="4" height="4"/>
                            →  16  6d 00 0a 70 00 2e 00 70 00 6e 00 67 00
                                   1a 00 00 00 04 00  1b 00 00 00 04 00  e8 03 fe
  ```

  Note also that the `#` is kept in `xlink:href` but stripped in
  `fill="url(#g)"`.

Two further cautions:

* Because attribute values are self-describing only to a reader that knows the
  layout, the two bytes `e8 03` occur **inside** values as well: `dur="1s"` is
  the millisecond count 1000, which is `e8 03 00 00`. A parser must consume
  values by layout, never scan for the terminator.
* The `begin` and `end` records contain bytes this survey could not account
  for, including 32-bit words that look like leaked pointers
  (`<animate begin="1s"/>` produced
  `6e 00 00 01 e8 03 00 00 e8 e9 12 00 99 26 42 00 08 75 00 6e 00 69 00 64 00 b0 3e ff 01 …`,
  where `08 75 00 6e 00 69 00 64 00` is an 8-byte string with an internal
  default value). The bytes were identical across three runs on this host, but
  whether they are stable on another machine is **unknown**. Do not attempt to
  reproduce animation timing byte-for-byte from this document.

### 4.10 Path and point data

`d` (id `0x004f`) and `points` (id `0x004e`) share one payload:

1. a 16-bit little-endian **command count**;
2. that many one-byte command codes;
3. a 16-bit little-endian **value count**;
4. that many numbers (4 bytes each).

Command codes:

| Code | Command | Values consumed |
| --- | --- | --- |
| 0 | move to | 2 |
| 1 | line to | 2 |
| 2 | quadratic curve to | 4 |
| 3 | cubic curve to | 6 |
| 4 | close path | 0 |

Everything is converted to absolute coordinates and to this reduced command set
before encoding. Verified conversions, all at `-v 3` and all shown as the bytes
after the id:

| `d` | Bytes |
| --- | --- |
| `M 1 2` | `01 00` `00` `02 00` `00 00 01 00 00 00 02 00` |
| `M 1 2 L 3 4` | `02 00` `00 01` `04 00` + 4 numbers 1,2,3,4 |
| `M 1 2 L 3 4 Z` | `03 00` `00 01 04` `04 00` + 4 numbers 1,2,3,4 |
| `m 1 2 l 3 4 z` | `03 00` `00 01 04` `04 00` + numbers 1,2,**4,6** (relative made absolute) |
| `M0 0 H 5` | `02 00` `00 01` `04 00` + 0,0,5,0 (horizontal becomes a line) |
| `M0 0 V 5` | `02 00` `00 01` `04 00` + 0,0,0,5 |
| `M0 0 C 1 2 3 4 5 6` | `02 00` `00 03` `08 00` + 0,0,1,2,3,4,5,6 |
| `M0 0 S 1 2 3 4` | `02 00` `00 03` `08 00` + 0,0,**0,0**,1,2,3,4 (smooth cubic expanded with the reflected control point, here the current point) |
| `M0 0 Q 1 2 3 4` | `02 00` `00 02` `06 00` + 0,0,1,2,3,4 |
| `M0 0 T 1 2` | `02 00` `00 02` `06 00` + 0,0,0,0,1,2 |
| `""`, `Z`, or anything containing an `A`/`a` arc | `00 00` `00 00` — the whole path is emptied |

**Elliptical arcs are not supported.** A single `A` command discards the entire
path, including the commands before it (`M0 0 L1 1 A 1 2 3 0 1 4 5` gives
`4f 00 00 00 00 00`). A `<path>` with no `d` attribute emits no attribute at
all (`1e e8 03`).

`points` is converted to the same representation: `<polyline>` becomes a move
plus lines, `<polygon>` additionally gets a close.

```
<polyline points="1,2 3,4 5,6"/>  →  20  4e 00  03 00  00 01 01  06 00  + 1,2,3,4,5,6
<polygon  points="1,2 3,4"/>      →  1f  4e 00  03 00  00 01 04  04 00  + 1,2,3,4
```

A 71-command path (`M` plus 70 `L`) encodes as command count `47 00`, 71
command bytes, value count `8e 00` (142) and 142 numbers, confirming both counts
are 16-bit.

### 4.11 `transform`

All transform functions are multiplied into a single 2×3 matrix and written as
six numbers in the order **a, c, e, b, d, f** (the two matrix rows: `a c e`
then `b d f`), followed by a 4-byte little-endian kind word.

| Input | Six numbers | Kind |
| --- | --- | --- |
| `translate(5)` | 1, 0, 5, 0, 1, 0 | `01 00 00 00` |
| `translate(1,2)` | 1, 0, 1, 0, 1, 2 | `01 00 00 00` |
| `scale(2)` | 2, 0, 0, 0, 2, 0 | `02 00 00 00` |
| `scale(2,3)` | 2, 0, 0, 0, 3, 0 | `02 00 00 00` |
| `translate(1,2) scale(2)` | 2, 0, 1, 0, 2, 2 | `03 00 00 00` |
| `rotate(90)` | 0, −1, 0, 1, 0, 0 | `06 00 00 00` |
| `matrix(1,2,3,4,5,6)` | 1, 3, 5, 2, 4, 6 | `07 00 00 00` |

The kind word is a bit set: 1 = the matrix has a translation, 2 = it has a
scale, 4 = it has a rotation/skew. `matrix()` always sets all three. `skewX`
and `skewY` were not measured, so their exact kind value is **unknown**.

Full bytes for `<rect transform="translate(1,2)"/>` at `-v 3`:

```
21  42 00
    00 00 01 00   00 00 00 00   00 00 01 00
    00 00 00 00   00 00 01 00   00 00 02 00
    01 00 00 00
    e8 03 fe
```

### 4.12 Character data

`<text>` (and only `<text>`, as far as was tested) may be followed, after its
`E8 03`, by a text block: the marker `FD`, one length byte in bytes, then
UTF-16LE. An empty `<text/>` still emits `fd 00`.

```
<text x="1">Hi</text>
→  19  31 00 01 00 00 01 00  e8 03  fd 04 48 00 69 00  fe
   text  x = one-item list, 1.0   (end)  "Hi"        /text
```

Note that `x` and `y` on `<text>` are lists: a one-byte item count precedes the
numbers, unlike on `<rect>`.

---

## 5. What the encoder accepts, drops and dies on

**Accepted and encoded:** the elements in §4.8 and the attributes in §4.9,
including nesting, groups, `defs`, gradients, `solidColor`, `use`, `image`,
`switch`, `a`, and the animation elements (whose value layouts are mostly
unknown).

**Silently dropped, with no diagnostic and exit code 0:**

* unknown elements (list in §4.2) and everything inside them;
* unknown attributes (list in §4.9);
* XML comments, the XML declaration, namespace declarations;
* `preserveAspectRatio` unless the value is exactly `none`;
* a `<path>` whose `d` contains an arc — the entire path data is emptied;
* `stroke="url(#…)"` — silently becomes black;
* numbers outside ±32765 and negative `width`/`height`;
* everything after the 127th character of a string (§4.7).

**Silently produces a broken file, exit code 0:** any input that is not
well-formed XML. `hello`, `<foo/>`, `` (empty file) and a document with a
duplicate attribute all give the 5-byte file `ce 56 fa 03 ff`. An unclosed
element (`<svg><rect x="1"></svg>`) gives a file whose element-end markers are
missing:

```
ce 56 fa 03 00 e8 03 21 31 00 00 00 01 00 e8 03 ff
```

**Crashes:** character data longer than 255 characters (segmentation fault, a
truncated file left on disk).

**Not investigated:** `<image>` with an embedded `data:` payload, external
references, `<font>`/`<glyph>` payloads, animation timing values.

---

## 6. The `.mif` container

### 6.1 Layout

All fields are 32-bit little-endian. There is **no alignment or padding
anywhere**: icon blobs simply abut, so an icon may start at an odd offset.

File header, 16 bytes:

| Offset | Field | Value observed |
| --- | --- | --- |
| 0 | signature | `42 23 23 34` — ASCII `B##4` |
| 4 | format version | 2 |
| 8 | offset of the entry table | always 16 |
| 12 | entry count | **2 × number of icons** |

Entry table, 8 bytes per entry, starting at offset 16:

| Offset in entry | Field |
| --- | --- |
| 0 | absolute file offset of the icon block |
| 4 | length of the icon block = 32 + length of the icon data |

For an SVG icon the entry is written **twice** — once for the icon and once for
its mask — and both copies point at the same block. This happens even when no
mask depth was requested (`/c32` alone still yields two identical entries), so
the entry count is always even and always twice the number of source icons.
Only the `.mbg` distinguishes the two cases.

Icon block, a 32-byte header followed by the data:

| Offset | Field | Value observed |
| --- | --- | --- |
| 0 | signature | `43 23 23 34` — ASCII `C##4` |
| 4 | icon-header version | 1 |
| 8 | header length | 32 (`0x20`) |
| 12 | data length in bytes | e.g. 193 |
| 16 | icon type | 1 for an SVG icon; the value for a bitmap icon is **unknown** |
| 20 | display mode (depth) | see the table below |
| 24 | animated flag | 1 when the source was preceded by `/A`, otherwise 0 |
| 28 | mask display mode | 0, 1 or 4 (below) |

The icon data follows immediately and is the raw `.svgb` file, magic included.
The first icon therefore starts at `16 + 8 × entryCount`; each subsequent icon
starts at the previous icon's offset plus its block length. The file ends with
the last icon's data; there are no trailing bytes.

### 6.2 Depth codes

From `/OPT` (Symbian display-mode numbering):

| `/OPT` depth | Field at +20 | `MASK` | Field at +28 |
| --- | --- | --- | --- |
| `/1` | 1 | (none) | 0 |
| `/2` | 2 | `1` | 1 |
| `/4` | 3 | `8` | 4 |
| `/8` | 4 | | |
| `/c4` | 5 | | |
| `/c8` | 6 | | |
| `/c12` | 10 | | |
| `/c16` | 7 | | |
| `/c24` | 8 | | |
| `/c32` | 11 | | |

So `/c32,8` — what the SDK example icon makefiles use — is depth 11, mask 4.

### 6.3 Verified example

`mifconv out_aif.mif /Hout_aif.mbg /S… /Tt /c32,8 icon.svg` on the symdev icon
template gives a 257-byte file:

```
0000  42 23 23 34  02 00 00 00  10 00 00 00  02 00 00 00   B##4, v2, table@16, 2 entries
0010  20 00 00 00  e1 00 00 00                             entry 0: offset 0x20, length 0xe1
0018  20 00 00 00  e1 00 00 00                             entry 1: identical (the mask)
0020  43 23 23 34  01 00 00 00  20 00 00 00  c1 00 00 00   C##4, v1, hdr 0x20, data 0xc1
0030  01 00 00 00  0b 00 00 00  00 00 00 00  04 00 00 00   type 1, depth 11, not animated, mask 4
0040  …193 bytes of .svgb…
```

A two-icon file built with `… /c32,8 a.svg /A /c24,1 b.svg` has entry count 4,
the table occupying offsets 16…47, the first icon block at 0x30 and the second
at 0x111 (= 0x30 + 0xe1, unaligned), and the second icon's header carries
depth 8, animated 1, mask 1.

Both files were rebuilt byte-for-byte from this description and compared
against the tool's output.

---

## 7. The `.mbg` header file

Plain ASCII with **CRLF** line endings throughout, including the last line. The
first line is a single space. The exact bytes for the example above:

```
20 0D 0A
2F 2A 20 54 68 69 73 …    /* This file has been generated, DO NOT MODIFY. */  0D 0A
enum TMifOut_aif                                                              0D 0A
09 7B                     a tab then {                                        0D 0A
09 EMbmOut_aifIcon = 16384,                                                   0D 0A
09 EMbmOut_aifIcon_mask = 16385,                                              0D 0A
09 EMbmOut_aifLastElement                                                     0D 0A
09 7D 3B                  a tab then };                                       0D 0A
```

Rendered:

```
<space>
/* This file has been generated, DO NOT MODIFY. */
enum TMifOut_aif
<tab>{
<tab>EMbmOut_aifIcon = 16384,
<tab>EMbmOut_aifIcon_mask = 16385,
<tab>EMbmOut_aifLastElement
<tab>};
```

Naming rules, all verified:

* **Stem transformation.** Take the file name without its directory, strip the
  final extension only, upper-case the first character and lower-case every
  other character. `out_aif.mif` → `Out_aif`; `icon.svg` → `Icon`;
  `UPPER.svg` → `Upper`; `bB_cc.svg` → `Bb_cc`; `9x.svg` → `9x`;
  `My.Icon.svg` → `My.icon` (only `.svg` is stripped, and the dot survives into
  the enumerator — the tool does not sanitise identifiers).
* The enum is named `TMif` + the transformed **mif** stem.
* Each icon contributes `EMbm` + the transformed mif stem + the transformed
  **source** stem.
* Values start at **16384** and increase by one in source order.
* An icon contributes a second enumerator, the same name with `_mask`
  appended, taking the next value — but **only when a mask depth was given**.
  With `/c32,8` both `EMbm…Icon = 16384` and `EMbm…Icon_mask = 16385` appear;
  with `/c32` only `EMbm…Icon = 16384` appears, even though the `.mif` still
  contains two entries.
* The list ends with `EMbm` + the transformed mif stem + `LastElement`, with no
  value and no trailing comma.

Three icons with `/c32,8` each give 16384/16385, 16386/16387, 16388/16389 and
then `LastElement`.

---

## 8. Worked example: the symdev icon template

Input (`examples/gui/gfx/gui.svg`, whitespace as shipped):

```
<?xml version="1.0" encoding="UTF-8"?>
<!-- symdev GUI template: app icon (SVG Tiny, built into gui_aif.mif). -->
<svg baseProfile="tiny" xmlns="http://www.w3.org/2000/svg" width="88" height="88" viewBox="0 0 88 88">
<rect x="4" y="4" width="80" height="80" rx="18" ry="18" fill="#1f6feb"/>
<rect x="22" y="22" width="44" height="30" rx="4" ry="4" fill="#ffffff"/>
<rect x="30" y="58" width="28" height="8" rx="4" ry="4" fill="#ffffff"/>
</svg>
```

Output at `-v 3`, 193 bytes:

```
0000  ce 56 fa 03  00  59 00 08 74 00 69 00 6e 00 79 00
0010  1a 00 00 00 00 58 00  1b 00 00 00 00 58 00  58 00
0020  00 00 00 00 00 00 00 00 00 00 58 00 00 00 58 00
0030  e8 03  21  31 00 00 00 04 00  30 00 00 00 04 00  1a
0040  00 00 00 50 00  1b 00 00 00 50 00  1d 00 00 00 12
0050  00  1e 00 00 00 12 00  00 00 00 eb 6f 1f 00  e8 03
0060  fe  21  31 00 00 00 16 00  30 00 00 00 16 00  1a 00
0070  00 00 2c 00  1b 00 00 00 1e 00  1d 00 00 00 04 00
0080  1e 00 00 00 04 00  00 00 00 ff ff ff 00  e8 03  fe
0090  21  31 00 00 00 1e 00  30 00 00 00 3a 00  1a 00 00
00a0  00 1c 00  1b 00 00 00 08 00  1d 00 00 00 04 00  1e
00b0  00 00 00 04 00  00 00 00 ff ff ff 00  e8 03  fe  fe
00c0  ff
```

Reading it:

| Bytes | Meaning |
| --- | --- |
| `ce 56 fa 03` | header, version 3 |
| `00` | `<svg>` |
| `59 00 08 74 00 69 00 6e 00 79 00` | `baseProfile`, string of 8 bytes, `tiny` |
| `1a 00 00 00 00 58 00` | `width`, unit byte 0, 88.0 |
| `1b 00 00 00 00 58 00` | `height`, unit byte 0, 88.0 |
| `58 00` + 16 bytes | `viewBox` = 0, 0, 88, 88 |
| `e8 03` | end of `<svg>` attributes |
| `21` | `<rect>` |
| `31 00 00 00 04 00` | `x` = 4 |
| `30 00 00 00 04 00` | `y` = 4 |
| `1a 00 00 00 50 00` | `width` = 80 (no unit byte here) |
| `1b 00 00 00 50 00` | `height` = 80 |
| `1d 00 00 00 12 00` | `rx` = 18 |
| `1e 00 00 00 12 00` | `ry` = 18 |
| `00 00 00 eb 6f 1f 00` | `fill`, literal flag 0, `0x001F6FEB` |
| `e8 03 fe` | end of attributes, end of `<rect>` |
| … | the two white rectangles, `fill` = `ff ff ff 00` = `0x00FFFFFF` |
| `fe ff` | end of `<svg>`, end of document |

At `-v 1` and `-v 4` the same file is also 193 bytes; only byte 0 and the
numbers change (`width="88"` becomes `00 00 b0 42`), and at `-v 1`/`-v 2` the
colour word is byte-reversed (`#1f6feb` → `1f 6f eb 00`).

Wrapped by `mifconv … /c32,8`, this is the 257-byte `.mif` of §6.3 and the
`.mbg` of §7.

Note what the template does **not** exercise and therefore what a minimal native
encoder needs: only the `<svg>` element with `baseProfile`, `width`, `height`,
`viewBox`, and `<rect>` with `x`, `y`, `width`, `height`, `rx`, `ry`, `fill`.
`<circle cx cy r fill>` is one more element token and two more ids:

```
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10">
  <circle cx="5" cy="5" r="4" fill="#ffcc00"/>
</svg>

ce 56 fa 03  00
58 00 00 00 00 00 00 00 00 00 00 00 0a 00 00 00 0a 00
e8 03
1b  2e 00 00 00 05 00  2f 00 00 00 05 00  1c 00 00 00 04 00
    00 00 00 cc ff 00
e8 03  fe  fe  ff
```

---

## 9. Left unspecified

* Element tokens `0x05`, `0x06`, `0x0e`, `0x2b`, `0x2c`.
* Attribute ids not listed in §4.9, and the value layouts marked unknown there
  — chiefly the animation attributes (`begin`, `end`, `restart`, `calcMode`,
  `keyTimes`, `keySplines`, `values`, `from`/`to`/`by`), `attributeName`, and
  the sync/media attributes. `min` and `max` on `<animate>` are dropped
  entirely.
* The enum mappings for `text-anchor` and `text-decoration`.
* The default attribute record that `<animateMotion/>` emits on its own.
* Unit byte values other than 0 (user units) and 1 (percent).
* The icon-type field value for bitmap icons in the `.mif` icon header, and
  everything about how `mifconv` drives `bmconv` for `.bmp` sources (the
  `_mask.bmp` / `_mask_soft.bmp` convention is visible in the tool but was not
  exercised).
* Whether `svgtbinencode` accepts more than one input file.
* `skewX`/`skewY` transform kind values.
* Whether any `.svgb` consumer cares about the fields this encoder never
  writes.
