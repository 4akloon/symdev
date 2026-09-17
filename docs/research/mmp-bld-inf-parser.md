# `.mmp` / `bld.inf` parser specification

M1 parser contract. **Specification only** — no parser crate in §17. M1 coding remains unauthorized until a later review after a hand-built `.sisx`.

Facts are **Verified** (promoted from spec §13 / §4.5 / §4.2), **Likely**, **Unknown**, or **Needs experiment**. Do not treat Likely as Verified. Do not fill gaps with guessed directives.

Cites: `docs/superpowers/specs/2026-09-16-symdev-m0-north-star-design.md` §13 (outline), §9.6, §20. Related: [uids-capabilities-signing.md](uids-capabilities-signing.md), [pipeline-and-tools.md](pipeline-and-tools.md).

## Scope

M1 on this path builds **EXE / ARMV5 / UREL / GCCE** only.

Parser output is a **structured model** for a Rust driver that invokes `g++` directly. The parser **must not** emit `abld` / `makmake` / `Makefile` output.

This document does not specify `rcomp` / `epocrc` argv. `START RESOURCE` is parsed as a block; emitting and compiling resources still shells out to legacy `rcomp`/`epocrc` in Wave 0/1.

## Fail closed

Unknown directives are **parse errors**. Do not silently ignore them.

A name is unknown if it is not on the support list for that file kind. Support lists below are **exact**. Do not add directives here because they appear in SDK samples; a later experiment that proves hello-on-E52 needs one requires a **new spec**.

Reject-list names are parse errors. Do not silently ignore.

`#if` with an unimplemented macro is a parse error, **not** silent false.

## `.mmp` — support (exact)

These names only:

`TARGET`, `TARGETTYPE`, `UID`, `TARGETPATH`, `SOURCE`, `SOURCEPATH`, `SYSTEMINCLUDE`, `USERINCLUDE`, `LIBRARY`, `STATICLIBRARY`, `CAPABILITY`, `EPOCSTACKSIZE`, `EPOCHEAPSIZE`, `EPOCALLOWDLLDATA`, `START RESOURCE` … `END`.

| Directive | Model |
|---|---|
| `TARGET` | Output basename |
| `TARGETTYPE` | Must be `EXE`. Any other value is a reject (below) |
| `UID` | Optional. If present: two or three UIDs, as in existing MMP practice |
| `TARGETPATH` | Install/target path string, retained |
| `SOURCE` | Source file names, retained |
| `SOURCEPATH` | Source directory strings, retained |
| `SYSTEMINCLUDE` | System include paths, retained |
| `USERINCLUDE` | User include paths, retained |
| `LIBRARY` | Link libraries, retained |
| `STATICLIBRARY` | Static libraries, retained |
| `CAPABILITY` | Capability names, retained. Self-sign policy is [uids-capabilities-signing.md](uids-capabilities-signing.md) (six user-grantable only); that check is the driver/manifest, not a new MMP directive |
| `EPOCSTACKSIZE` | Stack size, retained |
| `EPOCHEAPSIZE` | Heap size, retained |
| `EPOCALLOWDLLDATA` | Flag, retained |
| `START RESOURCE` … `END` | Zero or more resource blocks. Inner lines of each block are part of that block, not top-level directives |

`TARGETTYPE`: only `EXE` accepted.

`UID` is optional. If present: two or three UIDs, as in existing MMP practice. Mapping to `elf2e32 --uid1` / `--uid3` is M1. uid1 for EXE is `0x1000007a` (Verified, pipeline fragment). uid3 from the manifest is Verified as the product identity. **Needs experiment** for uid2 (and for how MMP `UID` fields map onto `--uid1`/`--uid3` beyond those facts).

`START RESOURCE` is parsed as a block. Resource compile/emit is not specified here.

How `SOURCEPATH` applies to following `SOURCE` lines: **Needs experiment**. Until recorded, retain both in appearance order. Do not invent a path dialect.

## `bld.inf` — support (exact)

These names only:

`PRJ_PLATFORMS`, `PRJ_EXPORTS`, `PRJ_MMPFILES`, `PRJ_TESTMMPFILES`, `#if` / `#else` / `#endif`.

| Directive | Model |
|---|---|
| `PRJ_PLATFORMS` | Platform tokens. Evaluation below |
| `PRJ_EXPORTS` | Export entries, retained |
| `PRJ_MMPFILES` | MMP paths to build (M1 default set) |
| `PRJ_TESTMMPFILES` | MMP paths, parsed, **not** in the M1 default build set |
| `#if` / `#else` / `#endif` | Conditional inclusion of following lines |

`#elif`, `#ifdef`, `#ifndef`, and `#include` are **not** on this list. They are unknown → parse error.

### `PRJ_PLATFORMS`

We only build **ARMV5 UREL GCCE**. Extra platforms listed are **ignored**.

- Directive **omitted** → default ARMV5 UREL GCCE.
- Directive **present** and does not include a platform we can interpret as that build → **error**.
- Directive **present** and includes a token we can interpret as that build → accept; ignore extra platforms.

Which tokens (`ARMV5`, `GCCE`, `UREL`) appear in real FP2 `bld.inf` files: **Needs experiment**. Until experiment 12 records them, a present `PRJ_PLATFORMS` whose tokens cannot be interpreted as ARMV5 UREL GCCE is an error (fail closed). Do not guess alias tables.

### `PRJ_TESTMMPFILES`

Parsed so we do not choke. M1 default build does **not** compile test MMPs unless a future flag says so. §17 does not define that flag. This spec does not define it either.

### `#if` / `#else` / `#endif`

`#if` syntax is supported. Which macros the SDK defines (`__GCCE__`, etc.): **Needs experiment**.

Until that list is recorded: an unimplemented macro in `#if` is a **parse error** (fail closed), not silent false.

Do not invent a default macro table. Experiment 12 unblocks evaluation rules.

## Reject (exact)

Parse error. Do not silently ignore. Do not add these to the support lists above.

- `TARGETTYPE` other than `EXE` (`DLL`, `LIB`, `EXEDLL`, `IMPLIB`, …).
- `OPTION`, `OPTION_GCCE`, `MACRO`, `ARMFPU`, `SMPSAFE`, `DEFFILE`, `NOSTRICTDEF`, `VENDORID`, `SECUREID`, `AIF`, `LANG`, `SOURCEEXPORT` — unless a later experiment proves hello-on-E52 requires one of them; then a **new spec** adds it.
- `PRJ_EXTENSIONS`, `PRJ_TESTEXPORTS`.
- `abld` / `makmake` / `Makefile` generation as an output of the parser.
- Directives that exist only for other language or platform tracks.

## Lexical (Likely, Needs experiment)

Line continuation, comments (`//` and `/* */`), and case-insensitivity of directive names: **Likely** but **Needs experiment**.

Starting point to record against (not Verified):

- Directive names: case-insensitive ASCII
- Comments: `#` and `//`

Record mismatches. Until recorded, M1 may implement that starting point; a mismatch against an FP2 file is an experiment outcome, not a silent skip.

`#if` / `#else` / `#endif` are preprocessor directives, not comments. A `#` line that is not one of those three is a comment under the starting point **or** an unknown directive if the token is a name — **Needs experiment**. Do not treat an unknown name after `#` as false/`#if 0`.

## Structured model (not a Makefile)

Output is data, not generated build files.

From a `bld.inf`: platform interpretation (ARMV5 UREL GCCE or error), export entries, MMP paths from `PRJ_MMPFILES`, MMP paths from `PRJ_TESTMMPFILES` (present, marked not-built-by-default), with `#if` branches resolved or a parse error.

From each built `.mmp`: the support-list fields above. `TARGETTYPE` is `EXE`. `UID` is optional; if present, two or three values.

The M1 driver (not this spec, not §17) will feed that model to `arm-none-symbianelf-g++` / `ld` / `elf2e32`. Parser scope stops at the model.

## Unknown / Needs experiment

Do not invent values. Label `UNKNOWN — requires experiment` in implementation notes until recorded.

| Item | Status |
|---|---|
| FP2 `PRJ_PLATFORMS` tokens (`ARMV5`, `GCCE`, `UREL`) | **Needs experiment** (backlog 12) |
| Live `#if` macros in the FP2 SDK (`__GCCE__`, …) | **Needs experiment** (backlog 12) |
| `UID` ↔ `elf2e32` uid2 | **Needs experiment** |
| Comment syntax (`#`, `//`, `/* */`) | **Likely**, **Needs experiment** |
| Directive-name case-insensitivity | **Likely**, **Needs experiment** |
| Line continuation | **Likely**, **Needs experiment** |
| `SOURCEPATH` vs following `SOURCE` | **Needs experiment** |

## Out of scope

- Parser crate, tests, or golden corpus in §17
- Makefile / `abld` / `makmake` emission
- `rcomp` / `epocrc` argv
- Extra MMP/`bld.inf` names not listed under support
- Language or platform tracks other than C++ EXE / ARMV5 / UREL / GCCE
