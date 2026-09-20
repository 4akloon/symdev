# `elf2e32` options that symdev does not implement yet — a clean-room specification

**Provenance.** Written 2026-09-20 by a separate agent, in the spec role of a two-role clean-room
process. The rules below were derived from disassembly and decompilation of the SDK binary
`epoc32/tools/elf2e32.exe` (S60 3rd FP2, "Symbian Post Linker, Elf2E32 v 2.00 (505)"), from reading
the sources of the native Linux rebuild `elf2e32_next` (EPL-1.0), and from black-box runs of both
binaries on ARM ELF inputs built with the toolchain on this host. The repository owner approved the
disassembly for this purpose. The engineer who implements this document has seen neither the
disassembly nor the `elf2e32_next` sources; this file contains no code, no pseudo-code, and no
names or addresses taken from either. Every rule below that is marked *verified* was reproduced by
running one or both binaries and comparing the bytes they wrote; rules that could not be pinned
that way are marked **unknown** and must not be guessed at.

Throughout, "the SDK tool" means `elf2e32.exe` run under Wine, and "the rebuild" means
`elf2e32_next`. symdev's goldens come from the rebuild, so every place the two disagree is called
out explicitly; the divergences are collected in section 11.

---

## 1. Ground facts the rest of the document builds on

### 1.1 Header layout and the offsets used below

All offsets are byte offsets from the start of the E32 image file, all values little-endian. The
image begins with three back-to-back header structures; the names are the conventional Symbian E32
format names.

| Offset | Size | Field |
| --- | --- | --- |
| 0x00 | 4 | `iUid1` |
| 0x04 | 4 | `iUid2` |
| 0x08 | 4 | `iUid3` |
| 0x0c | 4 | `iUidChecksum` |
| 0x10 | 4 | signature, always the four bytes `EPOC` |
| 0x14 | 4 | `iHeaderCrc` |
| 0x18 | 4 | `iModuleVersion` |
| 0x1c | 4 | `iCompressionType` |
| 0x20 | 4 | tool version (major byte, minor byte, build u16) |
| 0x24 | 4 | `iTimeLo` |
| 0x28 | 4 | `iTimeHi` |
| 0x2c | 4 | `iFlags` |
| 0x30 | 4 | `iCodeSize` |
| 0x34 | 4 | `iDataSize` |
| 0x38 | 4 | `iHeapSizeMin` |
| 0x3c | 4 | `iHeapSizeMax` |
| 0x40 | 4 | `iStackSize` |
| 0x44 | 4 | `iBssSize` |
| 0x48 | 4 | `iEntryPoint` |
| 0x4c | 4 | `iCodeBase` |
| 0x50 | 4 | `iDataBase` |
| 0x54 | 4 | `iDllRefTableCount` |
| 0x58 | 4 | `iExportDirOffset` |
| 0x5c | 4 | `iExportDirCount` |
| 0x60 | 4 | `iTextSize` |
| 0x64 | 4 | `iCodeOffset` |
| 0x68 | 4 | `iDataOffset` |
| 0x6c | 4 | `iImportOffset` |
| 0x70 | 4 | `iCodeRelocOffset` |
| 0x74 | 4 | `iDataRelocOffset` |
| 0x78 | 2 | `iProcessPriority` |
| 0x7a | 2 | `iCpuIdentifier` (always `0x2001`, ARMv5, in everything observed) |
| 0x7c | 4 | `iUncompressedSize` |
| 0x80 | 4 | `iSecureId` |
| 0x84 | 4 | `iVendorId` |
| 0x88 | 8 | capability mask |
| 0x90 | 4 | `iExceptionDescriptor` |
| 0x94 | 4 | `iSpare2`, always zero |
| 0x98 | 2 | `iExportDescSize` |
| 0x9a | 1 | `iExportDescType` |
| 0x9b | … | `iExportDesc`, the first byte of the export description |

The smallest possible header is therefore **0x9c = 156 bytes**: one byte of export description is
always reserved at 0x9b even when there is no description. Section 6 gives the rule for headers
longer than that.

### 1.2 The baseline flag word

With no flag-bearing options at all, both tools write `iFlags` = `0x1200002a` for a program image
and `0x1200002b` for a library image. That value is the sum of

| Bit(s) | Value | Meaning | When set |
| --- | --- | --- | --- |
| 0 | `0x00000001` | image is a DLL | see section 2.4 |
| 1 | `0x00000002` | "entry points are not called" | always on |
| 2 | `0x00000004` | fixed-address EXE | `--fixedaddress` |
| 3 | `0x00000008` | ABI is ARM EABI | always on |
| 5 | `0x00000020` | entry point type EKA2 | always on |
| 8 | `0x00000100` | code unpaged | `--unpaged` |
| 9 | `0x00000200` | code paged | `--paged` |
| 10 | `0x00000400` | named symbol lookup data present | `--namedlookup` |
| 20–23 | `0x00f00000` | hardware floating point type | see section 3 |
| 24–27 | `0x0f000000` | header format, always `0x02000000` (format V) | always |
| 28–31 | `0xf0000000` | import format, always `0x10000000` (ELF-derived) | always |

Verified: `--paged` gives `0x1200022a`, `--unpaged` gives `0x1200012a`, `--defaultpaged` leaves
`0x1200002a`, `--fixedaddress` gives `0x1200002e`, `--namedlookup` on a DLL gives `0x1200042b`.
`--callentry` changes nothing (bit 1 is unconditional).

The SDK tool of this vintage does **not** accept `--debuggable` or `--smpsafe`
(`E1014: Option --debuggable is Unrecognized.`); the rebuild does, and would set `0x00000800` and
`0x00004000`. Nothing in the FP2 build chain passes them.

### 1.3 Fields that always differ between the two binaries

Every image compared in this document differs between the SDK tool and the rebuild in exactly these
places, and an implementation should mask them when diffing:

* `iHeaderCrc` at 0x14 (follows from everything else),
* the tool version at 0x20 — the SDK writes `02 00 f9 01` (2.00 build 505), the rebuild writes
  `03 00 02 00` (3.0 build 2),
* `iTimeLo`/`iTimeHi` at 0x24/0x28.

Where this document says two outputs are "identical" it means identical apart from those three
fields.

---

## 2. `--targettype`

### 2.1 The names the SDK tool accepts, and the five classes behind them

The SDK tool maps the target-type spelling onto one of six internal classes. The mapping is a plain
table; matching is case-insensitive. Anything not in the table produces
`elf2e32 : Warning: W1041: Unsupported Target Type '<name>'.` and then behaves as the default
(section 2.3) — it is a *warning*, not an error, and an image is still written.

| Class | Names accepted for it |
| --- | --- |
| 0 — import library | `LIB`, `LIBRARY`, `IMPLIB` |
| 1 — DLL | `DLL`, `STDDLL` |
| 2 — EXE | `EXE` |
| 3 — polymorphic DLL | `ANI`, `APP`, `CTL`, `CTPKG`, `FSY`, `LDD`, `ECOMIIC`, `PLUGIN`, `KDLL`, `KEXT`, `MDA`, `MDL`, `RDL`, `NOTIFIER`, `NOTIFIER2`, `TEXTNOTIFIER2`, `PDD`, `PDL`, `VAR` |
| 4 — EXE with exports | `EXEXP` |
| 5 — standard EXE | `STDEXE` |

Two spellings in the table deserve a note. The name for the media-driver target is stored in the
binary with mixed case (`MDl`), but since matching is case-insensitive that is indistinguishable
from `MDL`. There is no entry at all for `EPOCEXE`, `EXEDLL`, `PLUGIN3`, `VAR2`, `KLIB` or `NONE`:
those four all produce the W1041 warning. Verified by running each name.

The rebuild accepts a partly different set: it also knows `EPOCEXE` and `EXEDLL` (as EXE),
`PLUGIN3`, `VAR2`, `KLIB` and `NONE`, does **not** know `APP`, `CTL`, `CTPKG`, `ECOMIIC`, `MDA`,
`MDL`, `RDL`, `NOTIFIER` or `NOTIFIER2` (it reports those as deprecated or unrecognised and exits),
and refuses `PLUGIN` unless `--sysdef` is also given. For symdev the SDK table is the one to follow,
because that is what the SDK build chain feeds the tool.

### 2.2 What the class actually changes — and what it does not

This is the single most important finding in this section, and it contradicts the rebuild:

> **The SDK tool never derives a UID from the target type.** `iUid1`, `iUid2` and `iUid3` in the
> image are exactly the values of `--uid1`, `--uid2` and `--uid3`, and zero for any of them that is
> not given.

Verified by building the same three-export ARM DLL ELF with `--uid1=0x10000079 --uid3=0xe5d1b001`
and no `--uid2`, once for each of the 21 names in classes 1 and 3 and once for each of the six names
the table does not contain (`EPOCEXE`, `EXEDLL`, `PLUGIN3`, `VAR2`, `KLIB`, `NONE`). All 27 images
are 412 bytes and byte-identical apart from CRC and timestamp; every one of them has `iUid2` =
`0x00000000`. In particular `STDDLL` does **not** write
`0x20004C45`, `ANI` does not write `0x10003b22`, `TEXTNOTIFIER2` does not write `0x101fe38b`, and so
on. The polymorphic-DLL UID2 values and the STD UID2 come from the MMP/`bldmake` layer, which passes
them as `--uid2`; the post-linker only copies them.

The SDK tool also does not *correct* a UID. Giving `--uid1=0x1000007a` with `--targettype=DLL`
prints `UID1 should be set to 0x10000079 for DLL Generation` on stdout and then writes
`iUid1 = 0x1000007a` into the image anyway (verified); `iUidChecksum` at 0x0c follows the UID words
that were actually written. The same happens in reverse for an EXE class given a DLL UID1.

The rebuild, by contrast, overwrites `iUid1` with `0x10000079`/`0x1000007a` by class, overwrites
`iUid2` with `0x20004C45` for `STDDLL`/`STDEXE` and with the polymorphic UID for class 3, and
refuses some combinations outright. **This is a real divergence: for a `STDDLL` build with no
`--uid2`, the rebuild writes `20 00 4c 45`-worth of bytes at 0x04 that the SDK tool leaves as
zero.** symdev should follow the SDK tool and copy the options through unchanged; if it must stay
byte-compatible with existing rebuild goldens, it has to do so only for the target types those
goldens cover (`DLL` and `EXE`, where the two agree because the build chain passes matching UIDs).

What the class *does* change:

| Effect | Class 0 | Class 1 | Class 2 | Class 3 | Class 4 | Class 5 |
| --- | --- | --- | --- | --- | --- | --- |
| Writes an E32 image | no | yes | yes | yes | yes | yes |
| Sets the DLL flag bit | — | yes | no | yes | no | no |
| Keeps the ELF's exports | — | yes | **no** | yes | yes | yes |
| Requires `--uid1` | no | yes | yes | yes | yes | yes |
| Requires `--elfinput` + `--output` | no | yes | yes | yes | yes | yes |
| Requires `--definput` + `--dso` | yes | no | no | no | no | no |
| Requires `--defoutput` + `--dso` | — | **yes** | no | no | **yes** | no |

Notes on that table, all verified:

* Class 0 takes a `.def` and writes only a `.dso`; with no `--definput` it fails with
  `E1017: Missing options : --definput.`, and with no `--dso` with `E1017: Missing options : --dso.`.
  It does not need an ELF and does not write an image.
* Class 1 (`DLL`, `STDDLL`) is the only *library* class that insists on `--defoutput` and `--dso`;
  omitting either is `E1017`. Class 3 (`ANI` and friends) happily writes an image with neither.
* Class 2 discards exports. Running `--targettype=EXE` on an ELF that exports three functions gives
  `iExportDirCount = 0`, `iExportDirOffset = 0`, and an image 24 bytes shorter than the same ELF
  built as a DLL (388 vs 412 bytes, verified) — the export directory is simply not emitted. Classes
  4 and 5 keep the exports: the same ELF as `EXEXP` or `STDEXE` gives `iExportDirCount = 3` and 412
  bytes, with the EXE flag word `0x1200002a`.
* Class 4 additionally needs `--defoutput` *and* `--dso`; with only one of them it stops at `E1017`.

### 2.3 No `--targettype` at all

The SDK tool does not fail. With `--uid1`, `--uid3`, `--elfinput` and `--output` and no
`--targettype`, it wrote the same 5652-byte image as `--targettype=EXE` on the same ELF (verified).
The message `Target Type Not Specified.` exists in the binary but was not produced on that path.

### 2.4 The DLL flag bit comes from the target type, not from the ELF

The rebuild decides the DLL flag by looking for a `_E32Dll` static symbol in the ELF. The SDK tool
does not: it sets the bit for classes 1 and 3 and clears it for classes 2, 4 and 5, whatever the ELF
contains. Verified both ways:

* `--targettype=DLL` on an EXE-shaped ELF (entry point `_E32Startup`, no `_E32Dll`): flags
  `0x1200002b`, i.e. the DLL bit is set. The rebuild refuses that ELF as a DLL because it has no
  exports.
* `--targettype=EXE` on a DLL-shaped ELF: flags `0x1200002a`, DLL bit clear. The rebuild refuses
  because it sees `_E32Dll`, sets the DLL bit itself, and then its own validator objects that the
  UID1 is not the DLL UID.

### 2.5 Everything else a target type might have been expected to do

* No entry-point symbol is injected for any plugin class. The rebuild synthesises a `--sysdef` value
  for ECOM plugins (an ordinal-1 factory export); the SDK tool does not, and a `PLUGIN` build of a
  DLL ELF produced exactly the DLL image.
* `iEntryPoint` is always the ELF entry point minus the read-only segment's link address,
  independent of the target type. For the DLL used here that is `0x44`; for the EXE, `0x1098`.
* `--priority` only takes effect for EXE-class targets. On class 1 the SDK tool leaves
  `iProcessPriority` at 350 (`EPriorityForeground`) even with `--priority=high`; on class 2 it wrote
  450 for `high` and 250 for `background` (verified). **The rebuild applies `--priority` to DLLs as
  well** — it wrote 450 into the DLL header. Divergence.
* `--heap` and `--stack` are applied whatever the class (`--stack=0x4000` → `iStackSize = 0x4000`,
  `--heap=0x2000,0x200000` → `0x2000`/`0x200000` at 0x38/0x3c), even for a DLL where they are
  meaningless. Both tools agree.
* `iCpuIdentifier` was `0x2001` in every image either tool produced; there is no option that changes
  it.

---

## 3. `--fpu`

The SDK tool of this vintage accepts exactly two spellings, case-insensitively: `softvfp` and
`vfpv2`. Anything else, including `vfpv3` and `vfpv3d16`, is
`elf2e32 : Error: E1019: Argument '<value>' not permitted for option fpu.` and no image is written.

The value lands in bits 20–23 of `iFlags` and nothing else in the image changes. Verified on the
same EXE ELF, `--uncompressed`, 5652 bytes in every case:

| `--fpu` | bits 20–23 | resulting `iFlags` | SDK tool | rebuild |
| --- | --- | --- | --- | --- |
| absent | `0` | `0x1200002a` | ok | ok |
| `softvfp` | `0` | `0x1200002a` | ok | ok |
| `SoftVFP` | `0` | `0x1200002a` | ok | ok |
| `vfpv2` | `1` → `0x00100000` | `0x1210002a` | ok | ok |
| `VFPV2` | `1` → `0x00100000` | `0x1210002a` | ok | ok |
| `vfpv3` | `2` → `0x00200000` | `0x1220002a` | **E1019** | ok |
| `vfpv3d16` | would be `3` → `0x00300000` | — | **E1019** | **rejected** |

For `softvfp` and `vfpv2` the SDK tool and the rebuild produce identical images. For `vfpv3` only
the rebuild produces an image, with bits 20–23 = 2; there is no SDK reference for it, and no FP2
build chain emits it.

`vfpv3d16` is not reachable in either binary: the SDK tool has no entry for it, and the rebuild's
own option table has a typo that makes its entry unmatchable, so it reports
`Argument vfpv3d16 is not correct.` The encoding for it (bits 20–23 = 3) is **unknown** in the sense
that no run of either binary ever produced it; it is documented in the format but should not be
implemented from a guess.

Recommendation for symdev: accept `softvfp` and `vfpv2` and write bits 20–23 = 0 and 1; refuse
everything else with the SDK's wording. Accepting `vfpv3` (bits = 2) is safe and matches the rebuild
if a caller ever needs it, but nothing on an S60 3rd FP2 device does.

---

## 4. `--sid` and `--vid`

Both are plain 32-bit copies: `--sid` into `iSecureId` at 0x80, `--vid` into `iVendorId` at 0x84.
Nothing else in the image changes. Verified with `--sid=0x12345678`, `--vid=0x70000001`, both
together, and `--vid=0x101fb657`: in every case the two tools produced identical images.

The defaults:

| Case | `iSecureId` | `iVendorId` |
| --- | --- | --- |
| neither option | `iUid3` | `0` |
| `--sid=<n>`, n ≠ 0 | `n` | `0` |
| `--vid=<n>` only | `iUid3` | `n` |
| `--sid=0` | **`0` (SDK tool)** / `iUid3` (rebuild) | `0` |
| no `--uid3` either | `0` | `0` |

So `--sid` defaults to `--uid3` in both tools, and the only divergence is the explicit zero: the SDK
tool takes `--sid=0` literally and writes zero, while the rebuild treats zero as "not set" and falls
back to `iUid3`. Verified (SDK `iSecureId = 0x00000000`, rebuild `iSecureId = 0xe79e4cf9` from the
same argv).

There is no vendor-id fallback in either tool: `iVendorId` is zero unless `--vid` says otherwise.

`--uid1` is mandatory for every class that writes an image; without it the SDK tool stops at
`E1017: Missing options : --uid1.` An explicit `--uid1=0` is rejected differently, with
`E1015: Missing arguments for option : --uid1.` — the argument parser treats a zero value as a
missing one. `--uid2` and `--uid3` are optional and default to zero; `iUidChecksum` at 0x0c is
computed from whatever the three UID words end up being.

---

## 5. Priority of the remaining sections

Sections 6 (export description) and 7 (`.def` handling) are where the SDK tool and the rebuild
differ most, and section 6 is the one blocking real work.

---

## 6. The export description, including the 9–16 export case

### 6.1 What the export description is

When some ordinals of a DLL are *absent* (a `.def` entry marked `ABSENT`, whose export slot is
filled with the image's entry-point address instead of a real function), the V header describes the
holes. Three encodings exist:

| `iExportDescType` at 0x9a | Meaning |
| --- | --- |
| `0x00` | no holes; `iExportDescSize` is 0 and there is no description |
| `0x01` | full presence bitmap, one bit per ordinal |
| `0x02` | sparse bitmap: a meta-bitmap of which bitmap bytes are not `0xff`, followed by those bytes |

Let *n* be `iExportDirCount`. Define

* `bitmapSize = (n + 7) / 8` — the size of the full presence bitmap,
* `metaSize = (bitmapSize + 7) / 8` — the size of the meta-bitmap,
* `holeBytes` = the number of bitmap bytes that are not `0xff`.

The full bitmap has bit *i* (LSB first within each byte, byte `i / 8`) **set** when ordinal *i+1* is
present and **clear** when it is absent; the padding bits above ordinal *n* in the last byte are
set. So eleven ordinals with ordinal 3 absent give the two bytes `fb ff`.

The tool then chooses:

* `iExportDescType = 0x01`, `iExportDescSize = bitmapSize`, description = the full bitmap; unless
* `metaSize + holeBytes < bitmapSize`, in which case `iExportDescType = 0x02`,
  `iExportDescSize = metaSize + holeBytes`, and the description is the meta-bitmap (bit *j* set when
  bitmap byte *j* is not `0xff`, LSB first) followed, in ascending byte order, by each of those
  bitmap bytes.

This is a strict `<`: on a tie the full bitmap wins. Verified across fourteen builds (table in 6.3).

### 6.2 The rule symdev is missing: how the header grows, and what goes in the gap

Let `descLen = iExportDescSize`. The header is

* the fixed 0x9c bytes of section 1.1, whose last byte (0x9b) holds **the first byte of the
  description**, followed by
* an appended block of `roundUp(descLen - 1, 4)` bytes holding the **remaining** `descLen - 1`
  description bytes plus padding.

So

> `iCodeOffset = 0x9c + roundUp(descLen - 1, 4)`, equivalently `0x98 + roundUp(descLen + 3, 4)`,
> and equal to 0x9c when `descLen` is 0 or 1.

The loader's own consistency check is the second form: it requires
`0x98 + roundUp(iExportDescSize + 3, 4)` to equal `iCodeOffset` exactly — not less (that is
"gaps between export description and code sections") and not more (that is "code section begins in
export description").

**The padding bytes.** They depend on the description type, and this is where the rebuild is wrong.

* Type `0x02` (sparse): the SDK tool zeroes the whole appended block before filling it, so the
  padding is `0x00`. The rebuild does the same. **Both agree.**
* Type `0x01` (full bitmap): the SDK tool does **not** zero the block. The padding bytes are
  whatever the allocator handed it, and with this binary they are consistently `0xCD` — the
  uninitialised-heap fill of the C runtime it was linked against. Verified: `0xCD` in every one of
  the five full-bitmap cases with padding, in repeated runs of the same argv, and in both
  compressed and `--uncompressed` output. **The rebuild emits no padding at all here**, which is why
  it produces an unloadable image and then rejects its own output (section 6.4).

Because the padding is inside the header it is covered by `iHeaderCrc`, and it survives compression
(the header is never compressed). An implementation that writes `0x00` instead of `0xCD` produces a
perfectly loadable image — the loader only checks sizes — but it will not be byte-identical to the
SDK tool's. There is no third reference to break the tie: this is a case where symdev has to make a
choice and record it. Writing `0xCD` reproduces the SDK tool exactly and is what a byte-exact
reimplementation should do; writing `0x00` is defensible only if symdev's policy is "never copy
uninitialised memory", and then the divergence must be documented against the SDK.

### 6.3 Verified byte table

All rows: one DLL ELF, `--targettype=DLL`, `--uncompressed`, frozen `.def` with the given number of
ordinals and the given ABSENT positions, SDK tool output. "Header 0x98…`iCodeOffset`" is the raw
byte string starting at offset 0x98 — that is, `iExportDescSize` (2 bytes), `iExportDescType`
(1 byte), then the description and its padding.

| Ordinals | ABSENT ordinals | `bitmapSize` | `iExportDescSize` | `iExportDescType` | `iCodeOffset` | Header 0x98…`iCodeOffset` | File size |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 5 | 3 | 1 | 1 | `0x01` | 0x9c | `01 00 01 fb` | 404 |
| 6 | 3 | 1 | 1 | `0x01` | 0x9c | `01 00 01 fb` | 416 |
| 7 | 3 | 1 | 1 | `0x01` | 0x9c | `01 00 01 fb` | 424 |
| 9 | 3 | 2 | 2 | `0x01` | 0xa0 | `02 00 01 fb ff cd cd cd` | 448 |
| 11 | 3 | 2 | 2 | `0x01` | 0xa0 | `02 00 01 fb ff cd cd cd` | 468 |
| 14 | 3 | 2 | 2 | `0x01` | 0xa0 | `02 00 01 fb ff cd cd cd` | 500 |
| 18 | 3 | 3 | 2 | `0x02` | 0xa0 | `02 00 02 01 fb 00 00 00` | 540 |
| 19 | 3, 12 | 3 | 3 | `0x01` | 0xa0 | `03 00 01 fb f7 ff cd cd` | 544 |
| 26 | 3 | 4 | 2 | `0x02` | 0xa0 | `02 00 02 01 fb 00 00 00` | 620 |
| 28 | 3, 12, 20 | 4 | 4 | `0x01` | 0xa0 | `04 00 01 fb f7 f7 ff cd` | 632 |
| 33 | 3, 6, 12, 15, 20, 23, 28, 31 | 5 | 5 | `0x01` | 0xa0 | `05 00 01 db b7 b7 b7 ff` | 660 |
| 41 | 3 | 6 | 2 | `0x02` | 0xa0 | `02 00 02 01 fb 00 00 00` | 768 |
| 44 | 3, 12, 20, 28 | 6 | 5 | `0x02` | 0xa0 | `05 00 02 0f fb f7 f7 f7` | 788 |
| 45 | 3, 12, 20, 28, 36 | 6 | 6 | `0x01` | 0xa4 | `06 00 01 fb f7 f7 f7 f7 ff cd cd cd` | 796 |

Reading one row in full, the eleven-ordinal case (this is the case symdev refuses today):
`iExportDirCount = 0x0000000b`, `iExportDirOffset = 0x00000170`, `iCodeOffset = 0x000000a0`,
`iExportDescSize = 0x0002`, `iExportDescType = 0x01`, description `fb ff` (ordinal 3 absent, ordinals
12–16 do not exist so their bits are set), padding `cd cd cd`. The export slot for ordinal 3 holds
`iCodeBase + iEntryPoint`. The `.dso` names that ordinal `_._.absent_export_3`.

The same input compressed (the default, `iCompressionType = 0x101f7afc`) is 398 bytes with exactly
the same 0x98…0xa0 bytes — the header is outside the compressed region.

### 6.4 Where the rebuild fails, precisely

The rebuild appends the description as a block of exactly `descLen` bytes for a full bitmap, with no
padding, and as a block of `roundUp(descLen - 1, 4) + 1` bytes for a sparse bitmap. The sparse case
therefore always lands on the correct `iCodeOffset`; the full-bitmap case is correct only when
`descLen ≡ 1 (mod 4)`, i.e. `bitmapSize` of 1, 5, 9, … For every other full bitmap the rebuild
writes a header that is 1 to 3 bytes short, and then its own validator refuses the file:

* `--uncompressed`: `Failed to validate E32 image for valid E32Image. It has gaps between export
  description and code sections.`
* compressed: it fails earlier, in the size check on the compressed payload —
  `Set wrong size for decompression. Expected: 465 but got: 468` for the eleven-ordinal case.

Verified failures at 9, 11 and 14 ordinals (`bitmapSize` 2), at 19 ordinals (3), 28 (4) and 45 (6).
The 5-, 6-, 7-, 33-ordinal full bitmaps and every sparse bitmap succeed and match the SDK tool byte
for byte.

So the practical statement for symdev: **the 9–16 export range is not special.** What is special is
a *full* export-description bitmap whose length is not 1 more than a multiple of 4. The 9–16 range
is simply the smallest window where that always happens, because there `bitmapSize` is 2 and the
sparse form can never be shorter. Implement the general rule from 6.2 and all of these cases fall
out; there will then be no reference bytes from the rebuild for them, only from the SDK tool, and
the goldens in the table above.

Feeding the SDK tool's eleven-ordinal image back into the rebuild's own reader reports
`Export Description: Size=002, Type=01` / `fb ff` / `Export description consistent`, so the padded
layout is the one both the reader and the loader expect.

### 6.5 Images with no exports at all

If `iExportDirCount` is 0, both tools write `iExportDescType = 0x01` with `iExportDescSize = 0`,
`iExportDirOffset = 0` and `iCodeOffset = 0x9c`. This is not an EXE-only rule: a `--targettype=DLL`
build of an ELF with no exports gets the same type-1/size-0 description (verified — the resulting
DLL image was byte-identical to the EXE image of the same ELF except for the DLL flag bit at 0x2c).
The byte at 0x9b is zero in both tools in this case.

---

## 7. Exports, the `.def` file, and what ends up in the `.dso`

### 7.1 Which ELF symbols become exports

Unchanged from what symdev already implements, and the same in both tools: defined global symbols of
type function or object, with default or protected visibility, whose section index is a real section
(not `SHN_UNDEF` and below `SHN_ABS`), excluding names beginning `_ZTS`. Linker markers of type
`NOTYPE` are never exported. `--ignorenoncallable` made no observable difference in any test.

For class 2 (`EXE`) the export set is discarded entirely, whatever the ELF and whatever `--definput`
says (section 2.2).

### 7.2 `.def` input syntax

An input `.def` is a header line `EXPORTS` followed by one line per ordinal. The SDK tool **requires**
the `EXPORTS` line: without it the first export line is
`E1051: EXPORTS expected before first export entry : <file>[Line No=1][<line>].` The rebuild does not
check for it and accepts the file.

An export line is a name, `@`, an ordinal, and then zero or more keywords, optionally followed by a
comment introduced by `;`. Ordinals must be 1, 2, 3, … in file order; a gap or a swap is
`E1012: Ordinal number is not in sequence : <file>[Line No=<n>][<name>].` Both tools produce this
error, but **the line numbers differ by one**: the SDK tool counts the first line of the file as 1,
the rebuild as 0. A missing `@` is `E1052: @ Missing : <file>[Line No=<n>][<rest of line>].`

The keywords:

| Keyword | Effect on the image | Effect on the output `.def` | Effect on the `.dso` |
| --- | --- | --- | --- |
| `NONAME` | none | reproduced | none |
| `DATA <size>` | none | reproduced with the size | the symbol becomes an object of that size instead of a 4-byte function |
| `R3UNUSED` | none | reproduced | none |
| `ABSENT` | the export slot holds `iCodeBase + iEntryPoint` and the ordinal is a hole in the export description | reproduced | the symbol is renamed `_._.absent_export_<ordinal>` |
| `MISSING` | see below | — | — |

Verified: `R3UNUSED` and `DATA 4` on the same three-export DLL both produce an image identical to the
plain build (only CRC and timestamp differ); only the `.def` and, for `DATA`, the `.dso` change. In
the `.dso` the `DATA` entry becomes `OBJECT` with the stated size while the other two stay `FUNC`
size 4 — everything else in the file is unchanged.

`MISSING` is a marker the tools write as part of a comment line (`; MISSING:`) rather than a
keyword; a `.def` containing such a comment line is read back with the comment ignored, and the
entry below it is treated normally (verified: the image and the round-tripped `.def` were the same
as without the comment line).

**Named exports, i.e. an entry without `NONAME`.** The SDK tool accepts the line, treats it exactly
like a `NONAME` entry, and writes `NONAME` back into the output `.def` (verified: a three-ordinal
`.def` whose second line lacked `NONAME` produced an image identical to the all-`NONAME` build, and
an output `.def` in which all three lines carry `NONAME`). E32 images have no name-keyed export
table at all, so there is nothing else it could do. The error text `NONAME Missing` exists in the
binary but was not reached on this path. The rebuild simply ignores any line without `NONAME`, which
makes the following line's ordinal look out of sequence and the build fails — so symdev cannot take
a golden from the rebuild here, and should follow the SDK tool: parse the line, ignore the absence
of `NONAME`, and emit `NONAME` on output.

**Aliases (`Alias=Real`).** The SDK tool does not treat `=` specially: it looks up the whole token
before `=` as a symbol name and fails with
`E1036: Symbol <token> Missing from ELF File : <elf>.` The rebuild refuses the syntax with a message
asking the user to file a bug. Neither produces a usable alias, so symdev should reject the form.

**A `.def` symbol that the ELF does not define**, without `ABSENT`, is a hard error in both:
`E1036: Symbol <name> Missing from ELF File : <elf>.` (SDK) versus a list of missed frozen symbols
(rebuild). Both still write the output `.def` with the symbol marked `ABSENT` before stopping.

**A `.def` symbol marked `ABSENT` that the ELF *does* define** — the case symdev refuses today —
is a warning in both, but they then do opposite things:

* SDK tool: `Elf2e32: Warning: Symbol <name> absent in the DEF file, but present in the ELF file`,
  and it **keeps the symbol absent**. Verified on a three-export DLL with ordinal 1 marked `ABSENT`:
  the export slot for ordinal 1 holds `0x00008044` (`iCodeBase` 0x8000 + `iEntryPoint` 0x44) instead
  of the real `0x00008039`, `iExportDescSize = 1`, `iExportDescType = 1`, description byte `fe`,
  `iCodeOffset = 0x9c`, and the output `.def` still says `ABSENT`.
* rebuild: `Warning: <name> absent in the DEF file and --sysdef, but present in the ELF file.`, then
  it **clears** the absent flag — export slot `0x00008039`, `iExportDescType = 0`,
  `iExportDescSize = 0`, and the output `.def` drops the `ABSENT` keyword.

symdev should follow the SDK tool (keep it absent), and note that this makes the image differ from
what the rebuild would have written.

### 7.3 Output `.def` text

Both tools write `EXPORTS`, a `; NEW:` marker line before the first symbol that was not in the input
`.def`, one tab-indented line per ordinal in ordinal order, and a final blank line. Two differences:

* **Line endings.** The SDK tool writes CRLF (`0d 0a`) throughout; the rebuild writes LF — one extra
  byte per line, counting the `EXPORTS` line and the trailing blank line. A three-export `.def` is
  89 bytes from the SDK tool and 84 from the rebuild; an eleven-ordinal one is 263 against 250.
  symdev currently matches the rebuild.
* **Comments.** The SDK tool preserves an input comment verbatim (` ; my comment` stays
  ` ; my comment`); the rebuild re-adds its own `; ` separator so the comment grows a semicolon on
  every round trip (` ; ; my comment`). symdev matches the rebuild and avoids rewriting frozen lines,
  which sidesteps this.

The line body is `<name> @ <ordinal> NONAME`, then ` DATA <size>` if the entry is data, then
` R3UNUSED`, then ` ABSENT`, then ` ; <comment>`, in that order. A symbol the tool has decided is
missing from the ELF gets the line prefixed with `; MISSING:`.

### 7.4 `.dso` divergences (independent of the options above)

Three differences appear in *every* `.dso` the two tools write, and they matter because symdev's
`.dso` goldens come from the rebuild:

1. **`DT_SONAME` is the `--dso` argument verbatim.** The SDK tool stores the whole string it was
   given, including the drive letter and directory, so a Wine run with an absolute `Z:` path puts
   that whole path in the string table; the rebuild stores only the file name. Running the SDK tool
   with a bare relative `--dso=m.dso` makes the two agree. This is not an option semantics
   difference, it is a path-handling difference, and it accounts for almost all `.dso` size deltas
   seen when comparing the tools.
2. **The first version-symbol entry.** The version table has one 16-bit entry per dynamic symbol,
   and entry 0 (for the undefined symbol) is never assigned. The SDK tool leaves it uninitialised —
   `cd cd` with this binary, observed in every `.dso` it wrote — while the rebuild deliberately
   zeroes it. Two bytes.
3. **String-table padding.** The dynamic string table is padded up to a 4-byte boundary; the SDK
   tool pads with ASCII spaces (`0x20`), the rebuild with NUL. Verified by varying the `--dso` file
   name length: names needing 2, 1, 0 and 3 pad bytes gave exactly that many `0x20` bytes from the
   SDK tool and that many `0x00` bytes from the rebuild.

With the same relative `--dso` name and the same inputs, an SDK `.dso` and a rebuild `.dso` differ in
exactly those two places (2 + up to 3 bytes) and nowhere else — verified on a 3-export DLL, an
11-export DLL with one absent ordinal, and an import-library-only run from a `.def`.

---

## 8. A DLL with no exports, and `--definput` for an EXE

**DLL with no exports.** The SDK tool accepts it. `--targettype=DLL` on an ELF that exports nothing
gave a valid image with `iExportDirCount = 0`, `iExportDirOffset = 0`, `iExportDescType = 0x01`,
`iExportDescSize = 0`, `iCodeOffset = 0x9c`, the DLL flag set, and an accompanying `.def` whose
entire content is `EXPORTS` CRLF CRLF (11 bytes: `45 58 50 4f 52 54 53 0d 0a 0d 0a`) and a `.dso`
with one dynamic symbol (the undefined entry) and an empty ordinal area. The rebuild refuses with
`DLL Elf file has no exports! Check symbol(s) visibility!`. symdev should allow it and follow the SDK
layout; there is no rebuild golden.

**`--definput` with `--targettype=EXE`.** The SDK tool reads nothing from the file and produces the
plain EXE image — verified byte-identical (apart from CRC and timestamp) to the same build without
`--definput`: `iExportDirCount = 0`, `iExportDirOffset = 0`, `iExportDescType = 0x01`, 388 bytes for
the DLL-shaped ELF used here. No warning is printed. The rebuild does not get that far: it keeps the
ELF's DLL nature and stops in its own validator. symdev should accept `--definput` for an EXE and
ignore it.

For classes 4 and 5, `--definput` is *not* ignored — those classes keep their exports (section 2.2),
so a frozen `.def` behaves there exactly as it does for a DLL.

---

## 9. Relocations, imports, and ELF shapes

### 9.1 Which ARM relocation types are used

Both tools keep exactly five relocation types from the ELF's relocation sections and silently drop
everything else, including the null type:

| ARM relocation type | Decimal | Kept? |
| --- | --- | --- |
| `R_ARM_NONE` | 0 | dropped |
| `R_ARM_ABS32` | 2 | kept |
| `R_ARM_GLOB_DAT` | 21 | kept |
| `R_ARM_JUMP_SLOT` | 22 | kept |
| `R_ARM_RELATIVE` | 23 | kept |
| `R_ARM_GOT_BREL` | 26 | kept |
| anything else | — | dropped |

This is the complete set; the SDK tool's decision is a single five-way test with everything else
falling through to "drop". The two implementations agree exactly. symdev's refusal on an unknown
type is therefore stricter than both tools — the tools would ignore it — and switching to "ignore"
is what byte-compatibility requires if such a type ever appears.

The ARM type only decides whether a relocation is carried over. What goes into the E32 relocation
entry is decided by *which segment the target symbol lives in*, not by the ARM type: the 16-bit
entry is `(offset & 0x0fff) | type`, where type is `0x1000` for a symbol in the read-only segment,
`0x2000` for one in the read-write segment, and `0x3000` ("inferred") otherwise; `0x0000` is
reserved and indicates a bad entry. Relocations whose *place* is in the read-only segment go to the
code relocation section, those whose place is in the read-write segment to the data relocation
section.

A relocation section is: a 4-byte size (of everything after these two words), a 4-byte count of
relocation entries, then a sequence of blocks. Each block is a 4-byte page offset (page base minus
the segment's link address), a 4-byte block size (including these two words), then the 16-bit
entries; a block is padded with one zero 16-bit entry when its length would otherwise be odd in
words, and the whole section is a multiple of 4 bytes. Verified on a DLL whose code relocation
section reads `1c 00 00 00 0a 00 00 00 00 00 00 00 1c 00 00 00` followed by ten 16-bit entries — size
0x1c, ten relocations, one block at page offset 0, block size 0x1c.

### 9.2 An import whose relocation is not in the code segment

This is a real shape: a DLL with an initialised data word holding the address of a function imported
from another DLL produces an `R_ARM_ABS32` relocation at a data-segment address against an imported
symbol. Built and run through both tools with `--dlldata`:

* The rebuild refuses:
  `'<symbol>' : '<elf>' Import relocation does not refer to code segment.`
* The SDK tool **writes an image and says nothing**, and the image is wrong. It patched the data word
  with the import ordinal (the data section of the 436-byte image is the four bytes `81 02 00 00`,
  i.e. the encoded ordinal 0x281 with a zero addend in the high half), but the import block's fix-up
  offset — which the loader interprets as an offset into the *code* section — came out as
  `0x00000000`, and no data relocation section was emitted (`iDataRelocOffset = 0`). A loader
  following that entry would patch the first word of the code section instead of the data word.

So the SDK tool has no correct behaviour to copy here; it produces a silently broken image. symdev's
current refusal is the right thing, and this document recommends keeping it with the rebuild's
wording. What the *correct* encoding would be is **unknown** — the E32 import format has no way to
express a fix-up outside the code section.

### 9.3 ELF shapes that are and are not accepted

Checked by patching the ELF header of a working input and running both tools. They agree in every
case:

| Patch | SDK tool | rebuild |
| --- | --- | --- |
| `e_ident[EI_CLASS]` = 64-bit | `E1005: ELF file <f> is not 32 bit.` | `ELF file <f> is not 32 bit.` |
| `e_ident[EI_DATA]` = big-endian | `E1007: ELF file <f> is not Little Endian.` | `ELF file <f> is not Little Endian.` |
| `e_type` = `ET_REL` | `E1009: ELF file <f> is neither executable (ET_EXEC) or shared (ET_DYN).` | same text |
| `e_ident` magic corrupted | **accepted**, image written normally | **accepted** |
| `e_machine` = `EM_386` | **accepted**, image written normally | **accepted** |
| `e_flags` = 0 | **accepted**, image written normally | **accepted** |

So only three header fields are actually validated: the class byte, the data-encoding byte and
`e_type`. The messages `Invalid ELF magic in file`, `ELF file %s does not target ARM` and
`ELF file %s is not BPABI conformant` exist in the SDK binary but were not reachable from these
inputs; under what conditions they fire is **unknown**.

**An ELF with no writable `PT_LOAD`.** Every ELF the SDK linker script produces has one, even when it
is empty (file size 0, memory size 0, at the data link address). Removing it — by turning the second
`PT_LOAD` into `PT_NULL`, and again by shortening the program-header count — made both tools fail:
the SDK tool with `E1063: Fatal Error in Postlinker` and no output, the rebuild with a segmentation
fault. Neither supports the shape, so symdev's refusal is correct and there is nothing to implement.
An ELF whose writable `PT_LOAD` exists but is empty is the normal case and needs
`iDataSize = 0`, `iBssSize` from the segment's memory size, `iDataBase` from its link address, and
`iDataOffset = 0`.

---

## 10. Compression

### 10.1 The three methods

| Selection | `iCompressionType` at 0x1c | SDK tool | rebuild |
| --- | --- | --- | --- |
| default (no option) | `0x101F7AFC` (deflate) | ok | ok |
| `--compressionmethod=inflate` | `0x101F7AFC` | ok | ok |
| `--uncompressed` | `0x00000000` | ok | ok |
| `--compressionmethod=none` | `0x00000000` | ok | ok |
| `--compressionmethod=bytepair` | `0x102822AA` | ok | **aborts** |

`--uncompressed` and `--compressionmethod=none` produce byte-identical files; so do the default and
`inflate`. Verified on the same EXE: 3588 bytes deflated, 5652 uncompressed, 3941 byte-paired.

In all three cases the first `iCodeOffset` bytes — the whole header — are stored uncompressed, and
`iUncompressedSize` at 0x7c is the size of everything after the header before compression. Only the
bytes from `iCodeOffset` onwards are compressed, and the file size is `iCodeOffset` plus the
compressed length.

### 10.2 Byte-pair compression: the exact layout

This is the part with no rebuild golden at all: the rebuild aborts (heap corruption, `malloc(): corrupted top size`) before writing anything when `--compressionmethod=bytepair` is given, on both a 5652-byte EXE and a 412-byte DLL. The SDK tool is the only reference, and the format below was confirmed by decompressing its output and comparing byte for byte with the same build's uncompressed image (they matched everywhere except `iHeaderCrc`, `iCompressionType` and the timestamp).

The compressed payload is **two independent streams**, not one:

1. the first covers exactly `iCodeSize` bytes — the code section, including the export directory and
   any symbol-lookup section that lives inside it;
2. the second covers the remainder, `iUncompressedSize - iCodeSize` bytes — the data section, import
   section and relocation sections.

They are concatenated, with no alignment or padding between them, and nothing follows the second.

Each stream begins with a 10-byte index table header, which is **not** 4-byte aligned and must be
written packed:

| Offset in stream | Size | Field |
| --- | --- | --- |
| 0 | 4 | total size of this stream, header and index table included |
| 4 | 4 | decompressed size of this stream |
| 8 | 2 | number of pages |

Then follow that many 16-bit values, one per page, each the compressed size of that page; then the
pages themselves, back to back. The page count is `ceil(decompressedSize / 4096)`; every page but the
last covers exactly 4096 input bytes. The stream's total size equals `10 + 2 * pages + sum(pageSizes)`.

Verified example, a 5652-byte uncompressed EXE compressed to 3941 bytes with `iCodeOffset = 0x9c`,
`iCodeSize = 5196`, `iUncompressedSize = 5496`:

| Stream | Starts at | Total size | Decompressed | Pages | Page sizes |
| --- | --- | --- | --- | --- | --- |
| code | 156 | 3555 | 5196 | 2 | 2752, 789 |
| rest | 3711 | 230 | 300 | 1 | 218 |

`156 + 3555 + 230 = 3941`, and `10 + 2*2 + 2752 + 789 = 3555`.

A second verified example, a 412-byte uncompressed DLL compressed to 399 bytes, `iCodeSize = 212`,
`iUncompressedSize = 256`: code stream at 156, total 190, decompressed 212, 1 page of 178 bytes;
rest stream at 346, total 53, decompressed 44, 1 page of 41 bytes. Note that the second stream is
*larger* than its input — the index table is always written, so a byte-paired image of a small module
can be bigger than the uncompressed one in that part.

**The per-page format.** Each page is a classic byte-pair encoding with a token table in front:

1. one byte: the number of tokens, 0 to 255;
2. if the token count is zero, the rest of the page is the original data verbatim, and the page's
   compressed size is the original size plus one — this is what the compressor emits when the
   encoding would not shrink the page;
3. otherwise one byte: the *marker* byte, an escape used to emit a literal;
4. then the token table, in one of two forms:
   * fewer than 32 tokens: that many 3-byte groups, each being the token byte, then the first and
     second byte of the pair it expands to; the groups are sorted ascending by token byte;
   * 32 tokens or more: a 32-byte bitmap of which byte values are tokens (bit `b & 7` of byte
     `b >> 3`, LSB first), followed by two bytes per set bit, in ascending byte value, giving that
     token's pair;
5. then the compressed data. Each byte is either the marker — in which case the single byte after it
   is emitted literally and is not itself expanded — or a byte that is expanded recursively through
   the token table, first element before second, until it maps to itself. A page stops when its input
   is exhausted or 4096 output bytes have been produced.

Two verified page headers from the example above: the first code page begins `66 32 …` — 0x66 = 102
tokens, so the marker `0x32` is followed by the 32-byte bitmap form; the rest-stream page begins
`05 01 02 00 00 03 11 02 05 12 02 06 02 00 07 30 …` — 5 tokens, marker `0x01`, then the five 3-byte
groups `02 00 00`, `03 11 02`, `05 12 02`, `06 02 00`, `07 30 …`.

**Choosing a compression method.** Neither tool ever picks a method on its own: the default is
deflate and the only way to get anything else is the option. Neither falls back to "store" when the
compressed form would be larger; the byte-paired DLL above is proof, since one of its two streams
expanded and the tool wrote it anyway. Whether the SDK tool would behave differently if the *whole*
image expanded is **unknown** — no test input made that happen.

---

## 11. Divergence summary

Every place where the SDK tool and `elf2e32_next` disagree, and which one symdev's current goldens
follow.

| # | Topic | SDK tool | `elf2e32_next` | symdev today |
| --- | --- | --- | --- | --- |
| 1 | tool version at 0x20 | `02 00 f9 01` | `03 00 02 00` | rebuild |
| 2 | `iUid1` from `--targettype` | never changed, warning only | forced by class | rebuild (agrees for DLL/EXE) |
| 3 | `iUid2` from `--targettype` | never changed | forced for STDDLL/STDEXE and class 3 | rebuild |
| 4 | DLL flag bit | from the target class | from the `_E32Dll` symbol in the ELF | rebuild (agrees when they match) |
| 5 | target-type name table | see 2.1 | partly different set | rebuild |
| 6 | `--priority` on a DLL | ignored | applied | rebuild |
| 7 | `--fpu=vfpv3` | rejected | accepted, bits 20–23 = 2 | refuses |
| 8 | `--sid=0` | literal zero | falls back to `iUid3` | rebuild |
| 9 | full-bitmap export description padding | present, `0xCD` | **absent — broken image** | refuses |
| 10 | sparse export description padding | present, `0x00` | present, `0x00` | agrees |
| 11 | DLL with no exports | allowed | refused | refuses |
| 12 | `--definput` with an EXE | ignored | fails | refuses |
| 13 | `.def` entry without `NONAME` | accepted, written back with `NONAME` | line skipped, build fails | refuses |
| 14 | `.def` `ABSENT` on a symbol the ELF defines | stays absent | absent flag cleared | refuses |
| 15 | output `.def` line endings | CRLF | LF | rebuild |
| 16 | output `.def` comments | preserved | gain an extra `; ` each round trip | rebuild |
| 17 | `.def` error line numbers | 1-based | 0-based | rebuild |
| 18 | `.def` without an `EXPORTS` header | error `E1051` | accepted | rebuild |
| 19 | `.dso` `DT_SONAME` | the `--dso` argument verbatim | the file name only | rebuild |
| 20 | `.dso` version-table entry 0 | uninitialised (`cd cd`) | zero | rebuild |
| 21 | `.dso` string-table padding | ASCII spaces | NUL | rebuild |
| 22 | `--compressionmethod=bytepair` | works | aborts | refuses |
| 23 | import relocation outside the code segment | silently wrong image | refused | refuses |
| 24 | `--debuggable`, `--smpsafe` | not recognised | accepted, bits `0x800`/`0x4000` | — |

Rows 9, 11, 12, 13, 14, 22 and 23 are the ones where symdev needs a golden and the rebuild cannot
supply one; for those the SDK tool is the only reference, and the byte examples in this document are
the goldens.

---

## 12. What stayed unknown

* The encoding for `--fpu=vfpv3d16` (bits 20–23 = 3). Neither binary can be made to emit it.
* The conditions under which the SDK tool's `Invalid ELF magic`, `does not target ARM` and
  `is not BPABI conformant` diagnostics fire. Patching the corresponding ELF header fields did not
  reach them.
* The correct E32 encoding for an import whose relocation lies outside the code segment. The SDK
  tool writes a fix-up offset of zero and no data relocation, which cannot be right; the format
  appears to have no representation for it.
* Whether the SDK tool would fall back to storing an image uncompressed if the compressed form were
  larger overall. It does not do so per stream, but no input made the whole image expand.
* Whether the `0xCD` padding in a full-bitmap export description, and the `0xCD` version-table entry
  and space padding in the `.dso`, are stable across other builds of `elf2e32.exe`. They are stable
  for this one — repeated runs, five description sizes, several `.dso` sizes — but they look like
  uninitialised memory, so a different SDK could differ.
* The behaviour of `--targettype` names that the SDK tool's table lacks beyond the fact that they
  warn and fall through to the default; whether the default is genuinely class 2 or an unset class
  that behaves the same was not separated.
* `--sysdef`, `--customdlltarget` and `--excludeunwantedexports` were not exercised; nothing in
  symdev refuses on them yet.

---

## 13. Reproducing the byte examples

Everything above was produced on this host with the SDK tool under Wine and the native rebuild,
using ARM ELF inputs from the recorded experiment recipes (the `f<N>.elf` DLLs with 4…40 exported
functions, the three-export `mathlib.elf`, and the `hello.elf` EXE). The shape of a run is:

* the SDK tool needs `Z:`-prefixed absolute paths for every file option, and `--uid1` is mandatory;
* give `--linkas` the value that matches the ELF's `DT_SONAME`, and `--libpath` the SDK's
  `epoc32/release/armv5/lib`;
* add `--uncompressed` when comparing raw layout, and leave it off to check that the header bytes are
  unchanged by compression;
* keep the `--dso` argument a bare relative file name when comparing `.dso` files between the tools,
  otherwise the whole path ends up in `DT_SONAME` and every later offset shifts;
* mask offsets 0x14–0x17, 0x20–0x23 and 0x24–0x2b before diffing two tools' images.

To reproduce the export-description table, take an `f<N>.elf` and write a `.def` listing its exported
functions in name order with ordinals 1…, inserting extra entries with names that are *not* in the
ELF and the `ABSENT` keyword at the ordinals you want to be holes. The number of ordinals and the
positions of the holes are the only two things that matter.
