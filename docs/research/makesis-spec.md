# MakeSIS / SignSIS specification

Written 2026-09-20 by a separate agent from disassembly of `epoc32/tools/makesis.exe`
(S60 3rd Edition FP2 SDK, version reported as `4, 0, 0, 10`) combined with black-box runs
of that binary and of `signsis.exe` under Wine on this Linux host. The repository owner
approved disassembling the SDK binaries for this purpose. The engineer who implements from
this document did not see the disassembly and does not need to: every rule below was
re-confirmed by writing a small `.pkg`, running the SDK tool, and reading the resulting
bytes. Where a rule could not be confirmed it is marked **unknown** rather than guessed.
Byte examples are transcribed from real outputs of those runs.

---

## 1. Scope and conventions

This document describes:

* the `.pkg` input language in full, statement by statement;
* the SIS container each statement produces, field by field;
* the compression rule, the capability rule, the hashing and checksum rules;
* the command line of both tools, and their exit codes;
* the length limits, the silent truncations, and the inputs that crash rather than report.

All integers in the SIS container are little-endian. All strings in the container are
UTF-16LE with **no** byte-order mark and **no** terminating NUL. "Character" below means a
UTF-16 code unit (2 bytes).

Hex dumps are shown as raw byte sequences in file order.

---

## 2. The command line

### 2.1 makesis

Usage as the tool itself prints it:

```
MakeSIS [-h] [-i] [-s] [-v] [-d directory] pkgfile [sisfile]
```

| Option | Effect on behaviour and on output bytes |
|---|---|
| `-h` | Print the built-in `.pkg` format help page to standard output and exit 0. No file is read or written. |
| `-i` | Print the OpenSSL dual-licence text to standard output and exit 0. No file is read or written. |
| `-v` | Verbose progress to standard output. **No effect on the output bytes.** |
| `-s` | Write a *stub* instead of a normal SIS (section 12). Changes the output bytes completely. |
| `-d<dir>` | Additional directory searched for source files that are not found relative to the current directory. **No effect on the bytes** other than which file is read. |

Flag letters are matched case-insensitively (`-D` behaves as `-d`). Flags may appear in any
order and may be interleaved with the positional arguments.

**`-d` takes its argument attached, not separated.** `-ddsrc` works; `-d dsrc` is rejected
with `unknown flag` and exit 1. A trailing backslash on the directory is accepted.
Both relative and `Z:\…`-style absolute directories work. When a source file is found
neither relative to the current directory nor under the `-d` directory the run fails with
`Cannot find file : <name>` followed by `file I/O fault.` and exit 1.

**Positional arguments.**

* The first positional argument is the `.pkg` file. Its name must end in `.PKG`
  (case-insensitive); otherwise `invalid source file`, exit 1. A name that ends in `.pkg`
  but does not exist gives `cannot open file, check filename and access rights`, exit 1.
* The second positional argument is the output file and is optional.
* Zero positional arguments gives `wrong number of arguments`, exit 1, plus the usage block.

**Output name validation (a real quirk — reproduce it or you will diverge).** An explicitly
given output name is accepted only when **both**:

1. its last four characters are `.SIS` compared case-insensitively, and
2. the whole name string is **longer than five characters**.

Verified: `ab.sis`, `q1.sis`, `aa.sis`, `12345.sis`, `dest/q.sis`, `./q.sis`, `dest/.sis`,
`AB.SIS`, `aB.SiS`, `sub\o.sis` all accepted. `q.sis`, `a.sis` (five characters) rejected.
`o.si`, `o`, `o.txt`, `o.sis2`, `x.sisx`, `ab.sisq`, `xy.sis.txt` rejected. The rejection is
`invalid destination file` plus the usage block, exit 1.

**Default output name.** When the second positional argument is omitted, the output is the
`.pkg` path with its extension replaced by **uppercase `.SIS`**. Verified: `pp.pkg` produced
`pp.SIS`, not `pp.sis`.

**`-s` also rewrites the extension.** With `-s`, whatever output name was given has its
extension replaced by uppercase `.SIS`: `-s t1.pkg stubout.sis` wrote `stubout.SIS`, and
`-s t1.pkg dest/foo.sis` wrote `dest/foo.SIS`. The name-length and `.SIS` validation above
still applies to the name as given.

**Verbose output.** With `-v` the tool prints, in order: a line about the byte-order mark,
a line naming the detected encoding, one `processing …` line per statement consumed, and
finally `Generating SIS installation file...` (or `Generating SIS stub file...` with `-s`).
Every diagnostic and progress line is prefixed `(1) : `. **The number in that prefix is
always 1**; it is not a line number. An error on the tenth line of a file still reports
`(1) :`. Do not try to reproduce line numbers — there are none.

Without `-v` a successful run prints nothing at all.

### 2.2 signsis

Usage as the tool prints it:

```
SignSIS [-?] [-c...] [-i] [-o[-p]] [-s] [-u] [-v] input [output [certificate key [passphrase]]]
```

| Option | Meaning |
|---|---|
| `-?`, `-h` | Print usage, exit 0. |
| `-cd`, `-cr` | Select the signing algorithm (DSA / RSA). |
| `-i` | Print licence information. |
| `-o` | Report on the contents of the SIS file, after any other operation. |
| `-p` | With `-o`, extract the certificates present. |
| `-s` | Sign (the default when a certificate and key are supplied). |
| `-u` | Remove the most recent signature. |
| `-v` | Verbose. |

`signsis` does **not** validate the output file name: `aa.sisx`, `a.sisx`, `bb.sis` and
`bb.foo` were all accepted and written under exactly the name given, in the given case.

### 2.3 Exit codes

| Code | Meaning |
|---|---|
| 0 | Success, including `-h` and `-i`. Also returned by `signsis` when the input is not a SIS file at all (it silently writes nothing — see section 13.6). |
| 1 | Any reported error. There is no distinct code per error class. |
| 192 | Process crashed without a diagnostic (section 15). Observed as the Wine exit status for an unhandled exception. |

Error text is printed to **standard output**, not standard error. Standard error carries
only Wine's own noise. `makesis` writes no output file on any error.

---

## 3. The `.pkg` file: lexical level

### 3.1 Text encoding

The file is decoded before parsing.

| Leading bytes | Treatment | Verified |
|---|---|---|
| `EF BB BF` | UTF-8 with BOM | accepted |
| `FF FE` | UTF-16LE with BOM | accepted |
| `FE FF` | UTF-16BE with BOM | accepted |
| anything else | assumed UTF-8 | accepted |

A UTF-16LE file **without** a BOM is read as UTF-8, and the interleaved NUL bytes then
either produce `unknown language specified` or `cannot convert file to unicode` /
`make sure .PKG and .TXT files are either UTF8 or UNICODE`, exit 1. An invalid UTF-8
sequence anywhere in the file gives the same conversion error, exit 1. A completely empty
file also gives the conversion error.

Non-ASCII UTF-8 content is carried through to the container unchanged: a package name
`nÄ` produced a 4-byte UTF-16 string.

Line endings may be LF, CRLF or bare CR; all three were accepted and produced identical
containers.

### 3.2 Whitespace, comments, statement dispatch

Leading and trailing whitespace on a line is ignored, and tabs are whitespace. Blank lines
are ignored. A `;` begins a comment that runs to end of line; it may stand alone on a line
or follow a statement.

A statement is recognised by its **first non-blank character**:

| First character | Statement |
|---|---|
| `&` | language list |
| `#` | package header |
| `%` | localised vendor names |
| `:` | non-localised (unique) vendor name |
| `=` | logo |
| `(` | dependency on another component |
| `[` | dependency on target hardware |
| `*` | signature (deprecated; see 4.11) |
| `"` | file line |
| `@` | embedded package |
| `{` | language block |
| `!` | options block |
| `+` | property line |
| `IF` | conditional block |
| `;` | comment |

Any other leading text is `unknown line`, exit 1. `ENDIF`, `ELSE` and `ELSEIF` outside an
`IF` are also `unknown line`.

The first eight forms in that table are only accepted at the outer level. The remaining
forms (`"`, `@`, `{`, `!`, `+`, `IF`, `;`) are *install-block* statements and may also
appear nested inside `IF` bodies and language blocks.

### 3.3 Keyword case

Statement keywords and option names are matched case-insensitively: `IF`/`if`,
`EXISTS`/`exists`, `PACKAGE`/`package`, `TYPE=`/`Type=` all work, and whitespace is allowed
around `=` in `TYPE = SA`.

**The *values* are case-sensitive.** `TYPE=sa` is a `syntax error`. Language codes and
install-type codes must be uppercase.

### 3.4 Universal string truncation

**Every** string the tool copies into the container is silently truncated to **254 UTF-16
characters (508 bytes)**. Verified separately for package names, localised vendor names,
the unique vendor name and file destinations: a 253-character destination emitted a 506-byte
string, a 254-character destination emitted 508 bytes, and 255, 256, 258 and 6008-character
destinations all emitted exactly 508 bytes. No warning is printed.

---

## 4. The `.pkg` statements

Throughout this section, *L* is the number of declared languages (see 4.1).

### 4.1 Language list — `&`

```
&aa[(dddd)],bb,...,zz
```

Two-letter codes, optionally each followed by a decimal dialect number in parentheses,
separated by commas. At most **one** language line is allowed anywhere in the file; a second
one is `the languages have already been defined`, exit 1.

**In practice the language line must come before the header.** The header's name count is
checked against the languages known at the moment the header is parsed, so a language line
placed after the header only works when it declares exactly one language: `#{"n"}` followed
by `&FR` is accepted and writes language id 2, but `#{"n"}` followed by `&EN,FR` is
`bad language count.` and `#{"n","m"}` followed by `&EN,FR` is `Expected } read ,`, both
exit 1.

If there is no language line at all, the tool assumes a single language, English, and — with
`-v` — prints `No languages defined, assuming English.`

**Encoding.** Each entry becomes one *SISLanguage* (type 11) element, in declaration order,
inside *SISSupportedLanguages* (type 15). The value written is

```
language id = base id of the two-letter code + dialect number
```

Verified: `&EN` → 1; `&EN(0)` → 1; `&EN(1)` → 2; `&EN(2)` → 3; `&EN(5)` → 6; `&EN(10)` → 11;
`&EN(100)` → 0x65; `&EN(1000)` → 0x3E9; `&EN(9999)` → 0x2710; `&FR(0002)` → 4.

An unrecognised code is `syntax error`, exit 1.

Base ids (105 codes accepted by this build):

```
EN=1  FR=2  GE=3  SP=4  IT=5  SW=6  DA=7  NO=8  FI=9  AM=10 SF=11 SG=12 PO=13 TU=14
IC=15 RU=16 HU=17 DU=18 BL=19 AU=20 BF=21 AS=22 NZ=23 IF=24 CS=25 SK=26 PL=27 SL=28
TC=29 HK=30 ZH=31 JA=32 TH=33 AF=34 SQ=35 AH=36 AR=37 HY=38 TL=39 BE=40 BN=41 BG=42
MY=43 CA=44 HR=45 CE=46 IE=47 SA=48 ET=49 FA=50 CF=51 GD=52 KA=53 EL=54 CG=55 GU=56
HE=57 HI=58 IN=59 GA=60 SZ=61 KN=62 KK=63 KM=64 KO=65 LO=66 LV=67 LT=68 MK=69 MS=70
ML=71 MR=72 MO=73 MN=74 NN=75 BP=76 PA=77 RO=78 SR=79 SI=80 SO=81 OS=82 LS=83 SH=84
FS=85 TA=87 TE=88 BO=89 TI=90 CT=91 TK=92 UK=93 UR=94 VI=96 CY=97 ZU=98 ME=100 ST=101
EA=129 YW=157 YH=158 YP=159 YJ=160 YT=161 MA=326
```

(86, 95 and 99 are not reachable from any code in this build.)

Byte example — `&EN` alone:

```
0f 00 00 00 14 00 00 00      type 15, length 20
02 00 00 00 0c 00 00 00      type 2 (array), length 12
0b 00 00 00                  element type 11
04 00 00 00 01 00 00 00      one element, length 4, value 1
```

### 4.2 Package header — `#`

```
#{"NAMEaa", ... "NAMEzz"},(UID), Major, Minor, Build [, Options]
```

* Exactly *L* quoted names must be given, one per declared language, in the language order,
  where *L* is the number of languages declared **before this line** (1 if there has been no
  `&` line yet). Too few is `Expected , read }`; too many is `Expected } read ,`. Both
  exit 1.
* The UID is parsed as C-style: `0xE1000001` or decimal `3774873648` both work.
* `Major`, `Minor` and `Build` are decimal. Any of them above **32767** is
  `The version numbers cannot be more than 32767.`, exit 1. 32767 is accepted.
* At most one header line; a second is `installation header already found`, exit 1.
* An install-block statement before the header is
  `the installation header has not been defined`, exit 1.
* A file with no header at all is `verification failure.`, exit 1.

**Options** is a comma-separated list. It may be omitted entirely (equivalent to `TYPE=SA`
with no flags).

`TYPE=` selects the install type, written as the first of the two trailing bytes of
*SISInfo*:

| `TYPE=` spelling | Long spelling | Install-type byte | Notes |
|---|---|---|---|
| `SA` | `SISAPP` | 0 | default |
| `SP` | `SISPATCH` | 1 | |
| `PU` | `PARTIALUPGRADE` | 2 | |
| `PA` | `PIAPP` | 3 | |
| `PP` | `PIPATCH` | 4 | |
| `SU` | `SISUPGRADE` | 0 | prints `Installation type SU ignored, application assumed.` |
| `SC` | `SISCONFIG` | 0 | same warning with `SC` |
| `SO` | `SISOPTION` | 0 | same warning with `SO` |
| `SY` | `SISSYSTEM` | 0 | same warning with `SY` |

Any other value is `syntax error`, exit 1. The four deprecated spellings warn on standard
output but still exit 0.

The remaining options are flags, written as the **second** trailing byte of *SISInfo*:

| Short | Long | Install-flags bit | Other effect |
|---|---|---|---|
| `SH` | `SHUTDOWNAPPS` | 0x01 | — |
| `NR` | `NONREMOVABLE` | 0x02 | — |
| `RU` | `ROMUPGRADE` | 0x04 | — |
| `IU` | `IUNICODE` | none (byte stays 0) | none observed |
| `NC` | `NOCOMPRESS` | none (byte stays 0) | forces stored, never deflated, for the controller **and** for every file data block (section 9) |

Flags combine: `TYPE=SA,RU,NR` produced `00 06`. No combination of install type and flag was
rejected; `RU` with `TYPE=SP`, `NR` with `TYPE=PA` and `RU` with `TYPE=PP` were all accepted
silently. The diagnostic
`Install flag option is not supported with the given install type.` exists in the tool but
**no input was found that triggers it — unknown.**

Byte example — trailing two bytes of *SISInfo*:

```
TYPE=SA (or no TYPE)          …  00 00
TYPE=SP                       …  01 00
TYPE=PU                       …  02 00
TYPE=PA                       …  03 00
TYPE=PP                       …  04 00
TYPE=SA,SH                    …  00 01
TYPE=SA,NR                    …  00 02
TYPE=SA,RU                    …  00 04
TYPE=SA,RU,NR                 …  00 06
```

The two bytes sit immediately after the *SISDateTime* field, inside the *SISInfo* payload,
and they are counted in the *SISInfo* length. They are not a separate TLV field and are not
padded beyond the enclosing field's own padding.

### 4.3 Localised vendor names — `%`

```
%{"Vendor-EN", ... "Vendor-FR"}
```

Exactly *L* quoted strings. This statement is **mandatory**: without it the run ends with
`bad language count.`, exit 1 (this is the same failure as omitting all vendor lines).

Encoded as an array of *SISString* inside *SISInfo*, in language order.

### 4.4 Unique vendor name — `:`

```
:"Symbian Software Ltd"
```

One quoted string. **Optional.** When absent, the vendor string in *SISInfo* is written as
an empty *SISString* (`01 00 00 00 00 00 00 00`) and the run still exits 0. When `-v` is
used and only one of the two vendor statements is present, a note is printed naming the
missing one.

### 4.5 File line — `"…"-"…"[,options]`

```
"Source"-"Destination"[,Option[,Option…]]
```

**Source.** Resolved relative to the current working directory (not to the `.pkg` file's
directory), then relative to the `-d` directory if given. Accepted forms, all verified:
plain name, `.\name`, `sub\name`, `sub/name`, `..\dir\name`, `Z:\absolute\path\name`, and a
Unix-style absolute path. Matching is case-insensitive in practice because the host
filesystem layer is. A missing source is `Cannot find file : <name>` plus `file I/O fault.`,
exit 1.

An **empty** source (`""`) means "no data": see 4.6.

**Destination.** Must be `X:\…` — a single drive letter, a colon, a backslash, then the
path. `!` is a valid drive letter and means "the drive the user chooses". Rejected with
`invalid destination path or syntax.`, exit 1:

* no drive (`\data\a.txt`),
* relative (`data\a.txt`),
* any `..` component,
* forward slashes (`!:/data/x.txt`),
* wildcards (`!:\data\*.txt`),
* the bare drive `!:` with nothing after it.

`!:\` alone is accepted. A destination ending in `\` is accepted and is written **literally**
— the tool does **not** append the source basename. The destination string is copied
verbatim, preserving case, then truncated to 254 characters (3.4).

**There is no special handling of executables.** A `.pkg` may contain any number of file
lines whose destination is under `\sys\bin\`, or none; the tool does not look at the
extension, does not require the executable's name to match the package name, and does not
treat the first executable differently from the rest. The only thing that distinguishes an
E32 file from any other file is the capability field of section 10, which is driven purely
by the content of the source file and not by its destination.

**Options.** Comma-separated, any order, case-insensitive. They set the *Operation* and
*OperationOptions* words of the *SISFileDescription*:

| Short | Long | Operation | OperationOptions bits | Notes |
|---|---|---|---|---|
| — (none) | — | 1 | 0 | default |
| `FF` | `FILE` | 1 | 0 | explicit default |
| `FR` | `FILERUN` | 2 | 0 | |
| `FT` | `FILETEXT` | 4 | 0 | |
| `FN` | `FILENULL` | 8 | 0 | |
| `FM` | `FILEMIME` | 2 | 0x08 \| 0x02 = 0x0A | takes an argument: `,FM,"mime/type"` |
| `MF` | `MODIFIABLE` | unchanged | 0x8000 | |
| `RI` | `RUNINSTALL` | 2 | 0x02 | |
| `RR` | `RUNREMOVE` | 2 | 0x04 | |
| `RB` | `RUNBOTH` | 2 | 0x06 | |
| `RW` | `RUNWAITEND` | unchanged | 0x10 | |
| `RS` | `RUNSENDEND` | unchanged | 0x20 | |
| `TS` | `TEXTSKIP` | unchanged | 0x400 | |
| `TA` | `TEXTABORT` | unchanged | 0x800 | |
| `TE` | `TEXTEXIT` | unchanged | 0x1000 | |
| `TC` | `TEXTCONTINUE` | unchanged | 0 | the default for `FT` |

`RI`, `RR`, `RB` and `FM` each set the operation to 2 by themselves; `FR` before them is
redundant. Verified combinations: `RB,RW` → operation 2, options 0x16; `RI,RW` → 0x12;
`RR,RW` → 0x14; `RI,RS` → 0x22; `FT,TA` → operation 4, options 0x800.

`FM` **must** be followed by a comma and a quoted MIME type; the MIME type is written as the
second *SISString* of the file description. `,FM "text/plain"` (no comma) is
`Expected , read quoted string`. `,FM,"text/plain",MF` and `,FT,FM,"text/plain"` are both
`syntax error`, exit 1 — `FM` with its argument must be the last thing on the line.

`FI` is **not** a file option (it is the Finnish language code); `,FI` is a `syntax error`.
The `F*` options are mutually exclusive: `,FN,FR`, `,FF,FT` and `,FT,FR` are all
`syntax error`, exit 1.

The diagnostic `RW/RUNWAITEND option required with RR/RUNREMOVE or RB/RUNBOTH options`
exists in the tool, but `,RR` and `,RB` without `RW` were both accepted silently, both in a
plain file line and in a language block. **The trigger is unknown.**

### 4.6 Directory and empty-file lines

Both use an empty source:

```
""-"!:\data\adir\"            ; create a directory
""-"!:\data\empty.txt",FN     ; a null file entry
```

They produce a *SISFileDescription* with:

* the destination string as given,
* an empty MIME string,
* a *SISHash* whose algorithm word is 1 and whose blob is **zero-length**
  (`19 00 00 00 0c 00 00 00 01 00 00 00 25 00 00 00 00 00 00 00`),
* `Length = 0`, `UncompressedLength = 0`, `FileIndex = 0`,
* the operation from the options (1 by default, 8 with `FN`).

**No entry is added to the data section.** The `FileIndex` of such a description is 0 even
when index 0 belongs to some other file; it is meaningless when the length is zero.

Note that a *non-empty* source with a destination ending in `\` is a normal file line, not a
directory line: it carries real data, a real hash, and the destination keeps the trailing
backslash.

### 4.7 Embedded package — `@`

```
@"Component.sis",(UID)
```

The named file must be an existing, well-formed SIS file whose package UID equals the UID in
parentheses, otherwise `different UID.`, exit 1. A file that is not a SIS at all is
`SISfile error.`, exit 1. A missing file is `Cannot open file : <name>` plus
`file I/O fault.`, exit 1.

**What happens to the bytes.** The embedded file is opened, its controller is inflated, and
that controller is inserted **byte for byte** as a *SISController* element in the enclosing
install block's controller array. The only byte that changes is the embedded controller's
own *SISDataIndex* value. Verified: for a 424-byte inner controller, the copy inside the
outer controller differed from the original in exactly one byte — offset 420, the low byte
of the trailing `SISDataIndex`, which went from 0 to 1.

The embedded file's 16-byte UID header, its `SISContents` wrapper and its two checksum
fields are discarded. Its data units are appended to the outer data array, and the embedded
controller's `SISDataIndex` is set to the index of its first data unit.

Embedding a SIS whose UID equals the outer package's own UID was **accepted silently**
(exit 0) in this build, although the tool contains a diagnostic for that case — treat that
diagnostic's trigger as unknown.

### 4.8 Dependencies — `(` and `[`

```
(UID),<versionrange>,{"DEPENDaa", ... "DEPENDzz"}     ; on another component
[UID],<versionrange>,{"DEPENDaa", ... "DEPENDzz"}     ; on target hardware
```

`<versionrange>` is either `major,minor,build` or `major,minor,build~major,minor,build`.
Each component may be a decimal number, `-1`, or `*`; `*` and `-1` both encode as `-1`
(`ff ff ff ff`).

The name list must contain exactly *L* strings. With one language declared, `{"A","B"}` is
`Expected } read ,`, exit 1. Omitting the name list entirely is
`Expected '-' or ',' in dependency line`, exit 1.

**Encoding.** *SISPrerequisites* (type 17) contains **two** arrays of *SISDependency*
(type 18), always both present even when empty:

1. the first array holds every `[UID]` target-hardware line, in file order;
2. the second array holds every `(UID)` component line, in file order.

Each *SISDependency* is a *SISUid* (type 9), a *SISVersionRange* (type 5), then an array of
*SISString*. A *SISVersionRange* contains one *SISVersion* (type 4, three signed 32-bit
words) for a single version and two for a range.

Byte example — `[0x102752AE],0,0,0,{"S60ProductID","S60FR"}` with two languages:

```
11 00 00 00 7c 01 00 00      type 17 Prerequisites
02 00 00 00 68 00 00 00      array, len 0x68
12 00 00 00                  element type 18
60 00 00 00                  element length 0x60
09 00 00 00 04 00 00 00 ae 52 27 10           Uid 0x102752AE
05 00 00 00 14 00 00 00                       VersionRange, len 20
  04 00 00 00 0c 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00   Version 0,0,0
02 00 00 00 30 00 00 00 01 00 00 00           array of strings
  18 00 00 00 53 00 36 00 30 00 50 00 72 00 6f 00 64 00 75 00 63 00 74 00 49 00 44 00
  0a 00 00 00 53 00 36 00 30 00 46 00 52 00 00 00
02 00 00 00 04 01 00 00 12 00 00 00 …         second array (component deps)
```

The empty case is 24 bytes:

```
11 00 00 00 18 00 00 00
02 00 00 00 04 00 00 00 12 00 00 00
02 00 00 00 04 00 00 00 12 00 00 00
```

### 4.9 Logo — `=`

```
="logo.jpg","image/jpeg"
="logo.jpg","image/jpeg","target.jpg"
```

Produces a *SISLogo* (type 23) containing a single *SISFileDescription*, placed in the
controller between *SISProperties* and *SISInstallBlock*. The logo's data is appended to the
data section like any other file and gets the next `FileIndex` (the logo statement being
before the file lines gives it index 0).

* Two-argument form: target string empty, MIME string set, `Operation = 2`,
  `OperationOptions = 0x0A`.
* Three-argument form: the third string becomes the target, and `Operation = 3`
  (install and run), `OperationOptions = 0x0A`.

### 4.10 Property line — `+`

```
+(key=value[,key=value]…)
```

Keys and values are numbers (decimal or `0x…`). Each pair becomes a *SISProperty* (type 20)
with an 8-byte payload: key word then value word. The properties go into *SISProperties*
(type 19), which is otherwise an empty array.

Byte example — `+(1=2,3=4)`:

```
13 00 00 00 24 00 00 00      type 19, length 0x24
02 00 00 00 1c 00 00 00      array, length 0x1c
14 00 00 00                  element type 20
08 00 00 00 01 00 00 00 02 00 00 00
08 00 00 00 03 00 00 00 04 00 00 00
```

Empty case (no `+` line), 12 bytes:

```
13 00 00 00 0c 00 00 00 02 00 00 00 04 00 00 00 14 00 00 00
```

### 4.11 Options block — `!`

```
!({"Opt1-EN", … "Opt1-zz"}[,{"Opt2-EN", … }]…)
```

Each brace group must contain exactly *L* strings. Each group becomes a *SISSupportedOption*
(type 33) whose payload is an array of *SISString* in language order. They fill
*SISSupportedOptions* (type 16), which is otherwise an empty array.

Byte example — `!({"O1"},{"O2"})` with one language:

```
10 00 00 00 3c 00 00 00      type 16, length 0x3c
02 00 00 00 34 00 00 00      array, length 0x34
21 00 00 00                  element type 33
14 00 00 00                  element length 20
  02 00 00 00 0c 00 00 00 01 00 00 00 04 00 00 00 4f 00 31 00
14 00 00 00
  02 00 00 00 0c 00 00 00 01 00 00 00 04 00 00 00 4f 00 32 00
```

Empty case (no `!` line), 12 bytes:

```
10 00 00 00 0c 00 00 00 02 00 00 00 04 00 00 00 21 00 00 00
```

### 4.12 Signature line — `*` (deprecated)

```
*"certificate","key"[,KEY="passphrase"]
```

The tool prints `Signature ignored,this option is deprecated...` on standard output and
**ignores the statement**. Parsing continues normally: file lines, dependencies and
everything else after a `*` line are still processed and still reach the output. The
statement contributes nothing to the output bytes, and the two named files are never opened
(a `*` line naming files that do not exist is accepted, exit 0).

Two quoted strings are required; one is `Expected , read quoted string`, exit 1. An optional
third element `,KEY="passphrase"` is accepted. Anything else after the second string is not
part of the statement and is re-parsed as a new statement, which usually fails.

A `*` line before the package header gives `the installation header has not been defined`,
exit 1.

### 4.13 Language block — `{ … }`

```
{
"en-source"
"fr-source"
}-"Destination"[,Options]
```

or, for components:

```
{
@"en-component.sis"
@"fr-component.sis"
},(UID)
```

Exactly *L* entries must appear between the braces, one per declared language, in language
order. An unterminated block is `Expected } read ?`, exit 1; a `}` with no `{` is
`unknown line`.

**Encoding.** A language block is *sugar for an `IF`/`ELSEIF` chain on the language
variable*. It produces one *SISIf* (type 26) whose expression is
`variable 0x1000 = <first language id>`, containing an install block with the first
language's entry, followed by one *SISElseIf* (type 27) per remaining language with the same
shape.

The variable 0x1000 is the language variable (section 5.3).

**A real difference from a plain file line:** a file introduced by a language block gets
`Operation = 0`, not 1, when no options are given. Verified with a two-language block and no
options: both branches carried `Operation=0 Options=0`. With options the normal mapping
applies (`,FR,RI,RW` gave `Operation=2 Options=0x12` in both branches). Reproduce the 0.

Each branch's source file gets its own data entry and its own `FileIndex` (they are distinct
sources, so the deduplication of section 8.3 does not merge them). For the component form,
each language's embedded controller gets its own data unit and its own `SISDataIndex`
(verified: outer index 0, the two embedded controllers 1 and 2, three data units total).

### 4.14 Conditional block — `IF` / `ELSEIF` / `ELSE` / `ENDIF`

```
IF condition
  …statements…
ELSEIF condition
  …statements…
ELSE
  …statements…
ENDIF
```

`IF` blocks nest (200 levels deep was accepted) and may contain any install-block statement,
including further `IF`s, language blocks and embedded packages. An empty body is legal. An
unterminated `IF` is `Expected ENDIF read ?`, exit 1.

**Encoding.** One *SISIf* (type 26) per `IF`, appended to the enclosing install block's
`If` array. Its payload is:

1. a *SISExpression* (type 29) — the condition;
2. a *SISInstallBlock* (type 28) — the body;
3. an array of *SISElseIf* (type 27), one element per `ELSEIF` **and one for `ELSE`**.

A *SISElseIf* payload is an expression followed by an install block.

**`ELSE` is encoded as an `ELSEIF` with a tautological expression**: operator 9 (logical NOT)
whose single child is operator 16 (number literal) with value 0. Byte-for-byte:

```
1d 00 00 00 18 00 00 00      type 29, length 24
09 00 00 00 00 00 00 00      operator 9, integer 0
1d 00 00 00 08 00 00 00      child: type 29, length 8
10 00 00 00 00 00 00 00      operator 16, integer 0
```

---

## 5. The condition expression language

### 5.1 Grammar as the tool accepts it

```
condition := variable
           | variable op number
           | variable op "string"
           | EXISTS("filename")
           | NOT(condition)
           | (condition)AND(condition)
           | (condition)OR(condition)
           | package(number) | devcap(number)
           | appcap(number,number) | appprop(number,number)

op := = | <> | > | < | >= | <=
```

`EXISTS` requires parentheses around its argument in this build, despite what the built-in
help page shows: `EXISTS "file"` is `Expected ( read quoted string`, exit 1;
`EXISTS("file")` and `EXISTS ("file")` both work. A bare variable with no operator is a
valid condition.

Numbers may be decimal, `0x…`, or negative; `-1` and `0xFFFFFFFF` both encode as
`ff ff ff ff`.

### 5.2 Expression encoding

A *SISExpression* (type 29) payload is:

```
operator word (u32)
integer value (u32)
then, depending on the operator, either
  - nothing,                                 (variable / number leaves)
  - one SISString (type 1),                  (string leaf, EXISTS)
  - one SISExpression (type 29),             (unary)
  - two SISExpressions (type 29).            (binary, two-argument functions)
```

There is no fixed slot for the string and no fixed slot for the children: a parser must
switch on the operator, and an encoder must emit only what that operator uses.

| Operator | Meaning | Payload after the two words |
|---|---|---|
| 1 | `=` | two sub-expressions |
| 2 | `<>` | two sub-expressions |
| 3 | `>` | two sub-expressions |
| 4 | `<` | two sub-expressions |
| 5 | `>=` | two sub-expressions |
| 6 | `<=` | two sub-expressions |
| 7 | `AND` | two sub-expressions |
| 8 | `OR` | two sub-expressions |
| 9 | `NOT` | one sub-expression |
| 10 | `EXISTS` | one *SISString* (the filename) |
| 11 | two-argument function (`appcap`, `appprop`) | two sub-expressions |
| 12 | one-argument function (`package`, `devcap`) | one sub-expression |
| 13 | string literal | one *SISString* |
| 15 | device variable | integer value holds the variable id, nothing follows |
| 16 | number literal | integer value holds the number, nothing follows |

The integer word is 0 for every operator except 15 and 16.

**`package` and `devcap` produce identical bytes** (both operator 12 with one numeric child),
and **`appcap` and `appprop` produce identical bytes** (both operator 11 with two numeric
children). The tool does not distinguish them. Verified.

Byte example — `IF MachineUid=0x20002233`, the complete *SISIf*:

```
1d 00 00 00 28 00 00 00      SISExpression, length 40
01 00 00 00 00 00 00 00      operator 1 (=), integer 0
1d 00 00 00 08 00 00 00      left:  SISExpression, length 8
0f 00 00 00 05 00 00 00        operator 15, variable id 5 (MachineUid)
1d 00 00 00 08 00 00 00      right: SISExpression, length 8
10 00 00 00 33 22 00 20        operator 16, number 0x20002233
1c 00 00 00 9c 00 00 00      SISInstallBlock, length 0x9c
…body…
02 00 00 00 04 00 00 00 1b 00 00 00     empty array of type 27 (ElseIf)
```

Byte example — `IF EXISTS("a")`:

```
1d 00 00 00 …
0a 00 00 00 00 00 00 00      operator 10, integer 0
01 00 00 00 02 00 00 00 61 00 00 00   SISString "a" (padded)
```

Byte example — `IF Manufacturer="abc"`:

```
01 00 00 00 00 00 00 00                        operator 1
1d 00 00 00 08 00 00 00 0f 00 00 00 00 00 00 00   left: variable 0
1d 00 00 00 … 0d 00 00 00 00 00 00 00 01 00 00 00 06 00 00 00 61 00 62 00 63 00
                                                right: operator 13 + string "abc"
```

### 5.3 Device variable ids

These are the names accepted after `IF`, with the integer written into the operator-15 leaf:

| id | name | id | name | id | name |
|---|---|---|---|---|---|
| 0 | `Manufacturer` | 30 | `KeyboardClickVolumeMax` | 55 | `MouseButtons` |
| 1 | `ManufacturerHardwareRev` | 31 | `DisplayXPixels` | 58 | `CaseSwitch` |
| 2 | `ManufacturerSoftwareRev` | 32 | `DisplayYPixels` | 61 | `LEDs` |
| 3 | `ManufacturerSoftwareBuild` | 33 | `DisplayXTwips` | 63 | `IntegratedPhone` |
| 4 | `Model` | 34 | `DisplayYTwips` | 64 | `DisplayBrightness` |
| 5 | `MachineUid` | 35 | `DisplayColors` | 65 | `DisplayBrightnessMax` |
| 6 | `DeviceFamily` | 38 | `DisplayContrastMax` | 66 | `KeyboardBacklightState` |
| 7 | `DeviceFamilyRev` | 39 | `Backlight` | 67 | `AccessoryPower` |
| 8 | `CPU` | 41 | `Pen` | 72 | `SystemDrive` |
| 9 | `CPUArch` | 42 | `PenX` | 101 | `FPHardware` |
| 10 | `CPUABI` | 43 | `PenY` | 103 | `NumHalAttributes` |
| 11 | `CPUSpeed` | 44 | `PenDisplayOn` | 4096 | `Language` |
| 14 | `SystemTickPeriod` | 45 | `PenClick` | 4097 | `RemoteInstall` |
| 15 | `MemoryRAM` | 48 | `PenClickVolumeMax` | | |
| 16 | `MemoryRAMFree` | 49 | `Mouse` | | |
| 17 | `MemoryROM` | 50 | `MouseX` | | |
| 18 | `MemoryPageSize` | 51 | `MouseY` | | |
| 21 | `PowerBackup` | | | | |
| 24 | `Keyboard` | | | | |
| 25 | `KeyboardDeviceKeys` | | | | |
| 26 | `KeyboardAppKeys` | | | | |
| 27 | `KeyboardClick` | | | | |

Names are matched case-insensitively. An unknown name is `syntax error`, exit 1.

---

## 6. The SIS container

### 6.1 Field framing

Every field is a TLV:

```
u32 type
u32 length          (payload bytes, excluding the header and excluding padding)
payload             (length bytes)
padding             (0..3 zero bytes, so the next field starts on a 4-byte boundary)
```

The next field begins at `8 + ((length + 3) & ~3)` from the current field's start. The
padding bytes are zero and are **not** counted in the length.

Arrays are field type 2. An array's payload is the **element type written once as a u32**,
followed, for each element, by that element's length word and payload (with the element's own
padding), but **not** the element's type word. An empty array's payload is just the 4-byte
element type. This means an empty array field occupies 12 bytes.

### 6.2 Field types used by these tools

| Type | Name used here | Payload |
|---|---|---|
| 1 | String | UTF-16LE, no BOM, no NUL |
| 2 | Array | element type word, then length+payload per element |
| 3 | Compressed | algorithm u32, uncompressed length u64, then the data |
| 4 | Version | three signed 32-bit words: major, minor, build |
| 5 | VersionRange | one or two Version fields |
| 6 | Date | u16 year, u8 month (0-based), u8 day (1-based) |
| 7 | Time | u8 hour, u8 minute, u8 second (3 bytes, padded to 4) |
| 8 | DateTime | a Date field then a Time field |
| 9 | Uid | u32 |
| 11 | Language | u32 language id |
| 12 | Contents | the whole body after the UID header |
| 13 | Controller | see 6.4 |
| 14 | Info | see 6.4 |
| 15 | SupportedLanguages | array of Language |
| 16 | SupportedOptions | array of SupportedOption |
| 17 | Prerequisites | two arrays of Dependency |
| 18 | Dependency | Uid, VersionRange, array of String |
| 19 | Properties | array of Property |
| 20 | Property | key u32, value u32 |
| 22 | CertificateChain | one Blob |
| 23 | Logo | one FileDescription |
| 24 | FileDescription | see 7.1 |
| 25 | Hash | algorithm u32 (always 1 = SHA-1), then a Blob |
| 26 | If | Expression, InstallBlock, array of ElseIf |
| 27 | ElseIf | Expression, InstallBlock |
| 28 | InstallBlock | array of FileDescription, array of Controller, array of If |
| 29 | Expression | see 5.2 |
| 30 | Data | array of DataUnit |
| 31 | DataUnit | array of FileData |
| 32 | FileData | one Compressed |
| 33 | SupportedOption | array of String |
| 34 | ControllerChecksum | 2 bytes |
| 35 | DataChecksum | 2 bytes |
| 36 | Signature | SignatureAlgorithm, Blob |
| 37 | Blob | raw bytes |
| 38 | SignatureAlgorithm | one String (an OID in dotted text) |
| 39 | SignatureCertificateChain | array of Signature, then a CertificateChain |
| 40 | DataIndex | u32 |
| 41 | Capabilities | u32 |

### 6.3 File layout

```
16-byte UID header
SISContents field (type 12) spanning the rest of the file
  SISControllerChecksum (34)
  SISDataChecksum (35)
  SISCompressed (3)      -- wraps the SISController field
  SISData (30)
```

The UID header is four little-endian words:

| Word | Value |
|---|---|
| UID1 | `0x10201A7A`, constant for every SIS this tool writes |
| UID2 | `0x00000000`, constant |
| UID3 | the package UID from the `#{}` header |
| checked | the EPOC UID checksum of the three words above |

The EPOC UID checksum is the pair of CCITT CRC-16 values (polynomial 0x1021, initial value
0, most-significant nibble first) over the odd-indexed and even-indexed bytes of the twelve
UID bytes: `checked = (crc16(odd bytes) << 16) | crc16(even bytes)`. Verified against four
files produced in this session.

Byte example — the first 40 bytes of a 336-byte SIS for UID3 `0xE1000001`:

```
7a 1a 20 10 00 00 00 00 01 00 00 e1 1b f8 d3 75     UID header
0c 00 00 00 38 01 00 00                             SISContents, length 0x138
22 00 00 00 02 00 00 00 27 b8 00 00                 ControllerChecksum
23 00 00 00 02 00 00 00 16 bc 00 00                 DataChecksum
```

`16 + 8 + length(SISContents)` equals the file size exactly.

### 6.4 Controller layout

The inflated *SISController* (type 13) payload holds these fields **in this order**. Note
that SupportedOptions comes **before** SupportedLanguages — that ordering is not what the
type numbers suggest.

| Order | Field | Always present? |
|---|---|---|
| 1 | Info (14) | yes |
| 2 | SupportedOptions (16) | yes, empty array when there is no `!` line |
| 3 | SupportedLanguages (15) | yes |
| 4 | Prerequisites (17) | yes, two arrays, both may be empty |
| 5 | Properties (19) | yes, empty array when there is no `+` line |
| 6 | Logo (23) | only when the `.pkg` has an `=` line |
| 7 | InstallBlock (28) | yes |
| 8 | SignatureCertificateChain (39) | only after signing; may repeat (section 13.3) |
| 9 | DataIndex (40) | yes, always last |

*SISInfo* (14) payload, in order: Uid (9), the unique vendor String (1), an array of the
package names (one String per language), an array of the localised vendor names, Version
(4), DateTime (8), then the **two raw bytes** install type and install flags.

---

## 7. Files and the install block

### 7.1 SISFileDescription

Payload, in order:

1. target *SISString* (the destination path),
2. MIME-type *SISString* (empty unless `FM` was used),
3. *SISCapabilities* (type 41) — **present only under the rules in section 10**,
4. *SISHash* (type 25),
5. `Operation` u32,
6. `OperationOptions` u32,
7. `Length` u64 — the number of bytes actually stored in the data block,
8. `UncompressedLength` u64 — the original file size,
9. `FileIndex` u32.

Nothing follows `FileIndex`.

*SISHash* is the algorithm word 1 followed by a *SISBlob* holding the **SHA-1 of the
uncompressed source file**, 20 bytes. Verified against independently computed SHA-1 for
several files. For an empty-source entry the blob is zero-length (section 4.6).

Byte example — an EXE with capabilities, destination `!:\sys\bin\caplo.exe`, 3588 bytes
stored uncompressed:

```
18 00 00 00 88 00 00 00                        FileDescription, length 0x88
01 00 00 00 28 00 00 00                        target string, 40 bytes
21 00 3a 00 5c 00 73 00 79 00 73 00 5c 00 62 00
69 00 6e 00 5c 00 63 00 61 00 70 00 6c 00 6f 00
2e 00 65 00 78 00 65 00
01 00 00 00 00 00 00 00                        empty MIME string
29 00 00 00 04 00 00 00 00 e0 0b 00            Capabilities 0x000BE000
19 00 00 00 20 00 00 00                        Hash, length 32
01 00 00 00                                      algorithm 1
25 00 00 00 14 00 00 00                          Blob, 20 bytes
3a 23 e7 e7 e6 0e d9 73 54 53 4b 2a 77 e5 65 cd
64 ea 39 70
01 00 00 00                                    Operation = 1
00 00 00 00                                    OperationOptions = 0
04 0e 00 00 00 00 00 00                        Length = 3588
04 0e 00 00 00 00 00 00                        UncompressedLength = 3588
00 00 00 00                                    FileIndex = 0
```

### 7.2 SISInstallBlock

Payload: exactly three arrays, always all three present even when empty, in this order:

1. array of *SISFileDescription* (element type 24),
2. array of *SISController* (element type 13) — embedded packages,
3. array of *SISIf* (element type 26).

Byte example of the empty tail of an install block:

```
02 00 00 00 04 00 00 00 0d 00 00 00     empty controller array
02 00 00 00 04 00 00 00 1a 00 00 00     empty If array
```

Install blocks nest: the body of every `IF`, `ELSEIF` and language-block branch is another
complete *SISInstallBlock* with the same three arrays.

Statement order inside a block is preserved in the corresponding array order.

---

## 8. The data section

### 8.1 Structure

```
SISData (30)
  array of SISDataUnit (31)
    array of SISFileData (32)
      SISCompressed (3)
```

There is **one data unit per controller**: index 0 is the package's own controller, and each
embedded controller gets the next index. The controller's trailing *SISDataIndex* (type 40)
names its data unit. Verified with one embedded package (indices 0 and 1) and with a
two-language component block (indices 0, 1, 2 and three data units).

Byte example — the whole data section of a package with one 4-byte file:

```
1e 00 00 00 38 00 00 00      SISData, length 0x38
02 00 00 00 30 00 00 00      array, length 0x30
1f 00 00 00                    element type 31
28 00 00 00                    DataUnit length 0x28
  02 00 00 00 20 00 00 00      array, length 0x20
  20 00 00 00                    element type 32
  18 00 00 00                    FileData length 0x18
    03 00 00 00 10 00 00 00      Compressed, length 16
    00 00 00 00                    algorithm 0 (stored)
    04 00 00 00 00 00 00 00        uncompressed length 4
    41 41 41 41                    the data
```

### 8.2 Ordering

Entries inside a data unit follow the order in which their `.pkg` statements first appear,
including statements inside `IF` bodies and language blocks, and including the logo.
`FileIndex` is the zero-based position in that unit's `SISFileData` array.

### 8.3 Deduplication is by source path, not by content

Two file lines with the **same source token** share one data entry and carry the same
`FileIndex`. Two file lines with **different source tokens** get separate entries even when
the files are byte-identical.

Verified: a package listing `a.txt`, `c.txt`, `a2.txt`, `a.txt`, `sub\d.txt`, `b.txt`, where
`a.txt` and `a2.txt` both contain `AAAA`, produced `FileIndex` 0, 1, 2, 0, 3, 4 and five
data entries.

Whether the comparison is on the raw source token or on some normalised form is **unknown**;
the safe rule is to key on the exact source string as written in the `.pkg`.

---

## 9. Compression

### 9.1 The rule

Each unit — the controller, and each file's data — is deflated, and the deflated form is kept
**only when it is strictly shorter than the original**. Otherwise the original bytes are
stored.

* `algorithm = 1` means the payload after the 12-byte prefix is a zlib stream.
* `algorithm = 0` means the payload after the prefix is the original bytes.

`SISCompressed` payload:

```
u32 algorithm (0 or 1)
u64 uncompressed length
then the stored or deflated bytes
```

The field length is therefore `12 + stored length`.

**Ties store.** Three inputs were constructed whose deflated length exactly equalled their
original length (118, 67 and 115 bytes); all three were written with `algorithm = 0`. The
comparison is `deflated < original`, not `<=`.

There is no size threshold and no minimum: a 16-byte all-zero file was compressed (11 bytes),
an 8-byte all-zero file was not (its deflated form is 11 bytes). A zero-byte file is stored
with `algorithm = 0`, uncompressed length 0, and no data bytes.

### 9.2 zlib parameters

**zlib format (not raw deflate), compression level 6, default window size 15, default
memory level, default strategy, no preset dictionary.** Streams begin `78 9c`.

Verified by byte comparison: for every compressed unit produced in this session, the stored
bytes equalled the output of a stock zlib compressing the same input at level 6, and did not
equal level 1 or level 9.

### 9.3 NOCOMPRESS

`NC` / `NOCOMPRESS` in the header options forces `algorithm = 0` for the controller **and**
for every file data block, whatever the sizes. Verified: an 80-byte highly compressible file
that is normally stored deflated at 12 bytes was stored as 80 raw bytes, and the controller
was stored raw as well.

### 9.4 Is the controller ever stored without NOCOMPRESS?

No case was found. A controller carrying a 6000-character near-random destination string
still deflated smaller. Whether the "strictly smaller" rule also governs the controller, or
whether the controller is unconditionally deflated when `NC` is absent, is **unknown**;
implementing the same "strictly smaller" test for the controller is consistent with every
observation.

---

## 10. Capabilities (type 41)

A *SISCapabilities* field is written into a *SISFileDescription* if and only if **all** of
these hold for the source file:

1. its first sixteen bytes parse as an E32 image header — specifically the four-byte
   signature at offset 0x10 must be the EPOC signature, and the UID checksum word at offset
   0x0C must be the correct EPOC UID checksum of the three UIDs at offsets 0x00, 0x04, 0x08;
2. the header CRC word at offset 0x14 must be correct — the EPOC image header CRC over the
   first `iCodeOffset` bytes of the file with that word temporarily set to the CRC
   initialiser;
3. the image header format field (bits 24–27 of the flags word at offset 0x2C) must be 2;
4. the low 32 bits of the capability set at offset 0x88 must be non-zero.

The field payload is **4 bytes: the low 32 bits of the capability set**. The high 32 bits at
offset 0x8C are discarded — an image with low word 1 and high word 2 produced
`29 00 00 00 04 00 00 00 01 00 00 00`.

Anything the tool cannot validate simply gets **no** field, with no warning: a text file, a
truncated file, an image with a wrong header CRC, an image with a wrong UID checksum, an
image with a corrupted signature word, an image whose header format field is 3, and an image
whose capability low word is 0.

**The destination does not matter.** An E32 installed to `!:\data\`, to `c:\sys\bin\` or to
`!:\sys\bin\sub\` all received the field. **UID1 does not matter either**: images with UID1
`0x1000007A`, `0x10000079`, `0x1000008D` and `0x00000000` all received the field once their
UID checksum and header CRC were corrected. The earlier folklore that only `!:\sys\bin\`
EXEs get capabilities is wrong; what actually filtered the DLL in a naive test was the
invalid UID checksum left behind by patching UID1.

Practical consequence for an implementation: read the capability word out of the source
file's own E32 header and validate that header exactly as above. Do not derive capabilities
from the `.pkg` or the destination.

Byte example (0x000BE000):

```
29 00 00 00 04 00 00 00 00 e0 0b 00
```

---

## 11. Checksums, timestamps

### 11.1 Checksums

*SISControllerChecksum* (34) and *SISDataChecksum* (35) each carry a 2-byte little-endian
value: the **EPOC CRC-16** (CCITT, polynomial 0x1021, initial value 0, high nibble first)
over the **complete field bytes** of, respectively, the *SISCompressed* field that wraps the
controller and the *SISData* field — header, payload and padding included.

Verified independently on three files produced in this session, signed and unsigned.

Both checksum fields have a 2-byte payload and are therefore 12 bytes on disk (8 header +
2 payload + 2 padding).

### 11.2 Creation time

*SISDateTime* holds the **UTC** wall-clock time of the run.

* *SISDate* payload: u16 year, u8 month **0-based** (0 = January), u8 day **1-based**.
* *SISTime* payload: u8 hour, u8 minute, u8 second, then one padding byte.

Byte example for 2026-09-20 10:01:30 UTC (the local clock read 12:01:30 in a UTC+2 zone):

```
08 00 00 00 18 00 00 00      DateTime, length 24
06 00 00 00 04 00 00 00      Date
ea 07 08 14                    2026, month 8 (September), day 20
07 00 00 00 03 00 00 00      Time
0a 01 1e 00                    10:01:30, one pad byte
```

This is the only non-deterministic part of a normal `makesis` run: two runs on the same
inputs differ only in these seven bytes and in everything downstream of them (the controller
deflate, the controller checksum, the container length, and therefore the file size).

---

## 12. The ROM stub (`-s`)

With `-s`, the output is **not** a SIS file. It is the *SISController* field on its own,
uncompressed, with:

* no 16-byte UID header,
* no *SISContents* wrapper,
* no checksum fields,
* no data section.

The file is exactly `8 + controller payload length` bytes. Verified: a package whose normal
SIS was 336 bytes with a 412-byte controller payload produced a 420-byte stub whose first
eight bytes were `0d 00 00 00 9c 01 00 00` (type 13, length 412) and whose remainder was the
controller payload.

The controller content is identical to the non-stub controller except for the timestamp:
**a stub uses a fixed creation time**, `d4 07 00 01` for the date and `00 00 00` for the
time — year 2004, month 0 (January), day 1, 00:00:00. This makes stub output deterministic.

The file descriptions still carry lengths, hashes and capabilities exactly as in the normal
output.

---

## 13. signsis

### 13.1 What it reads

`signsis` opens an existing SIS, reads the 16-byte UID header, walks the *SISContents*
children, and **inflates** the wrapped controller. It then rebuilds the file.

### 13.2 What it changes

Signing inserts a *SISSignatureCertificateChain* (type 39) into the controller payload,
**between the InstallBlock (28) and the DataIndex (40)**, re-deflates the controller with the
same parameters, recomputes *SISControllerChecksum*, and rewrites the *SISContents* length.

Everything else is left alone:

* the 16-byte UID header is byte-identical;
* the *SISData* field is byte-identical;
* *SISDataChecksum* is unchanged;
* the creation timestamp is unchanged;
* the controller's compression **algorithm is preserved** — signing a SIS built with
  `NOCOMPRESS` produced a signed SIS whose controller is still `algorithm = 0`.

**Round-trip is exact.** Signing an unsigned SIS and then running `signsis -u` on the result
produced a file byte-identical to the original input. That is the strongest available proof
that the re-deflate uses the same zlib settings as `makesis` (section 9.2) and that nothing
else is touched.

### 13.3 Multiple signatures

Signing an already-signed file **appends a second type-39 field** as a sibling, before the
DataIndex. It does not add a second element to the existing signature array. A file signed
twice has two type-39 fields and two certificate chains.

`-u` removes the most recent (last) one.

### 13.4 What is signed

The message is the concatenation of the controller payload's fields **up to but not
including the type-39 field being added**, and excluding the trailing type-40 DataIndex.
For the first signature that is Info + SupportedOptions + SupportedLanguages + Prerequisites
+ Properties + (Logo) + InstallBlock. For the second signature it is the same bytes **plus
the first type-39 field**.

Verified with a real DSA public key: the first signature verified over a 400-byte message,
the second failed over 400 bytes and verified over 1304 bytes (400 + the 904-byte first
type-39 field including its header and padding).

Each field is included with its header, payload and padding — exactly its on-disk bytes.

### 13.5 Type-39 layout

```
SISSignatureCertificateChain (39)
  array (2) of Signature (36)
    Signature (36)
      SignatureAlgorithm (38)
        String (1)     the algorithm OID in dotted decimal text
      Blob (37)        the signature value
  CertificateChain (22)
    Blob (37)          the DER certificate chain
```

For DSA the OID string is `1.2.840.10040.4.3` (34 bytes as UTF-16) and the hash is SHA-1.

**The signature blob is a fixed 48 bytes, zero-padded.** A DSA signature's DER encoding
varies between 44 and 48 bytes; the blob's declared length is 48 in every case, with zero
bytes filling the tail. Eight signings of the same file produced DER lengths 46, 46, 46, 46,
48, 46, 47, 46 — all in a 48-byte blob. An implementation must emit 48 and pad, not emit the
DER length.

Byte example of the head of a type-39 field:

```
27 00 00 00 80 03 00 00      type 39, length 0x380
02 00 00 00 74 00 00 00      array, length 0x74
24 00 00 00                    element type 36
6c 00 00 00                    element length 0x6c
26 00 00 00 2c 00 00 00        SignatureAlgorithm, length 0x2c
01 00 00 00 22 00 00 00          String, 34 bytes
31 00 2e 00 32 00 2e 00 38 00 34 00 30 00 2e 00
31 00 30 00 30 00 34 00 30 00 2e 00 34 00 2e 00
33 00 …                          "1.2.840.10040.4.3"
```

### 13.6 Errors

| Input | Behaviour |
|---|---|
| Missing input file | `file I/O fault, cannot open <name>.` exit 1 |
| Wrong passphrase | OpenSSL decrypt error text plus `Unexpected error.` exit 1 |
| `-u` on an unsigned SIS | `not signed, input file.` exit 1, no output written |
| Input that is not a SIS at all | **exit 0, nothing printed, no output file written** — a silent no-op |

The last row is a trap: a caller that checks only the exit status will believe it succeeded.

---

## 14. Length limits and silent truncation

| Thing | Limit | Behaviour past the limit |
|---|---|---|
| Any string written to the container | 254 UTF-16 characters | silently truncated, no warning |
| Version major/minor/build | 32767 | `The version numbers cannot be more than 32767.` exit 1 |
| Output file name | must be > 5 characters and end `.SIS` | `invalid destination file` exit 1 |
| Nesting depth of `IF` | at least 200 | no limit found |

---

## 15. Inputs that crash rather than report

**An E32 source file whose image-header format field is 0 or 1 while its ABI field is set
crashes `makesis`.** The process terminates with no diagnostic at all and writes no output;
under Wine the exit status is 192.

Narrowed by varying only the flags word at offset 0x2C of an otherwise valid E32 (with the
header CRC recomputed each time):

| Flags word | Result |
|---|---|
| `0x1200002A` (header format 2) | normal, capabilities written |
| `0x1300002A` (header format 3) | normal, no capabilities |
| `0x0000002A` (format 0) | **crash, exit 192** |
| `0x0100002A` (format 1) | **crash, exit 192** |
| `0x00000008` (format 0, ABI bit only) | **crash, exit 192** |
| `0x0000000A` | **crash, exit 192** |
| `0x00000000` | normal, no capabilities |
| `0x00000002` | normal, no capabilities |
| `0x00000020` | normal, no capabilities |

So the trigger is bit 3 of the flags word set while the header-format field is 0 or 1. A
native implementation should report this as a malformed-image error instead.

Everything else tried was reported cleanly: unterminated `IF`, unterminated `{`, stray
`ENDIF`/`ELSE`/`}`, unbalanced parentheses in a condition, an empty file, a file containing
only a BOM, mismatched language counts, unknown language codes, a self-referential embedded
package, a 6000-character destination, and 200-deep `IF` nesting.

Note that a crashed run can leave a Wine debugger process behind on this host.

---

## 16. Diagnostics observed

All of these go to standard output. Every one of them accompanies exit 1 unless noted.

Argument and file-level: `wrong number of arguments`, `unknown flag`, `invalid source file`,
`invalid destination file`, `cannot open file, check filename and access rights`,
`cannot convert file to unicode`, `make sure .PKG and .TXT files are either UTF8 or UNICODE`.

Parse-level, each prefixed `(1) : error: `: `unknown line`, `syntax error.`,
`unexpected text`, `Expected <token> read <token>`, `The version numbers cannot be more than
32767.`, `installation header already found`, `the languages have already been defined`,
`the installation header has not been defined`, `unknown language specified`,
`Expected '-' or ',' in dependency line`, `different UID.`, `SISfile error.`,
`file I/O fault.`, `invalid destination path or syntax.`, `bad language count.`,
`verification failure.`.

Non-fatal (exit 0): `Installation type <xx> ignored, application assumed.`,
`Signature ignored,this option is deprecated...`, `No languages defined, assuming English.`

---

## 17. What remains unknown

* Whether the "keep the deflated form only when strictly smaller" rule is applied to the
  controller as well, or whether the controller is unconditionally deflated when
  `NOCOMPRESS` is absent. No input was found where deflating the controller made it larger.
* What triggers `Install flag option is not supported with the given install type.` — every
  combination of install type and flag tried was accepted.
* What triggers `RW/RUNWAITEND option required with RR/RUNREMOVE or RB/RUNBOTH options` —
  `RR` and `RB` without `RW` were accepted everywhere they were tried.
* What triggers the diagnostic about an embedded SIS sharing the main component's UID —
  embedding a SIS with the same UID as the outer package was accepted silently.
* Whether source-file deduplication compares the raw source token or a normalised path.
* The exact semantics the installer attaches to operator 11 versus operator 12 in
  expressions; `makesis` collapses `package`/`devcap` into one and `appcap`/`appprop` into
  the other, so the `.pkg` cannot express the difference.
* Whether the 254-character truncation is applied before or after the destination-syntax
  check for destinations longer than 254 characters that would become invalid once cut.
* What `IU` / `IUNICODE` changes. It is parsed and accepted, and it left every byte of the
  output unchanged in every test.
* Whether a `Time` field ever carries a fourth meaningful byte; the fourth byte was always
  zero padding here.
