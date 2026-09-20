# A third-party app through symdev: Simon Tatham's Puzzles (S60 port)

Run 2026-09-20 against `dbc273a`. The question: what does symdev fail on when the input is
somebody else's real S60 3rd Edition project rather than `examples/`?

Target: [tdionizio/puzzless60](https://github.com/tdionizio/puzzless60) — Tiago Dionizio's port
of Simon Tatham's Portable Puzzle Collection (Symbian C++ Avkon front end over ~55 `.c` files of
game code, 34 bitmap icons, four TTF fonts, a 389-line `.rss`, 26 libraries, Open C `libc`/`libm`).
Nothing was committed from it; the working tree lived in the session scratchpad.

## Result

`symdev build` + a SIS packed from symdev's own `BuildOutputs`/`SisPackage` produced a 552 KB
self-signed `.sisx`. EKA2L1 (RM-469) installed it, the app launched, the game list drew its 34
bitmaps, **Cube** opened and played (`Moves: 5` after arrow keys). Emulator only — not a claim of
E52 support.

The port needed 9 edits to build. Every one of them exists because symdev refuses something the
SDK accepts, so each is a gap below, not a property of the app.

## What already works on real input

- **rcomp**: the 389-line `Puzzles.rss` (`.rls` include, menu bar, six menus, `AVKON_SETTING_ITEM_LIST`,
  `AVKON_SETTING_PAGE`, `POPUP_SETTING_LIST`, `EDWIN`, `LOCALISABLE_APP_INFO`) and the registration
  resource compiled first try, `.rsg` included cleanly by the C++ sources.
- **elf2e32**: 65 objects, 26 `LIBRARY` lines, `EPOCSTACKSIZE 0x14000`, `EPOCHEAPSIZE 0x40000 0x1000000`
  → a 383 KB E32 EXE that runs.
- **SIS + self-signing**: an 8-file package (EXE, two `.rsc`, two icon containers, two fonts) installs.

## Gaps, in the order the build hits them

| # | What happens | Where | Severity |
|---|---|---|---|
| 1 | `/* … */` header comment → `error: unknown directive: /*`. Carbide puts one in every generated `bld.inf` and `.mmp`. | `bld.rs`, `mmp.rs` | blocking, trivial |
| 2 | `#ifdef ENABLE_ABIV2_MODE` / `#include <platform_paths.hrh>` → `unsupported preprocessor`. The SDK runs both files through `cpp`. | `bld.rs`, `mmp.rs` | blocking |
| 3 | `MACRO`, `OPTION GCCE -O2 -s`, `SECUREID`, `VENDORID`, `LANG SC`, `DEBUGGABLE_UDEBONLY` → `unknown directive`. `MACRO COMBINED` / `MACRO STYLUS_BASED` are load-bearing: without them the game sources do not build. | `mmp.rs` | blocking |
| 4 | `TARGET Puzzles_0xa000ef77` inside `START RESOURCE` → `unknown directive`. It renames the `.rsc`/`.rsg`, which the sources `#include` by that name. | `mmp.rs` | blocking |
| 5 | `gnumakefile Icons_scalable_dc.mk` under `PRJ_MMPFILES` is silently collected as an MMP path and fails later with a confusing error. | `bld.rs` | high |
| 6 | `SYSTEMINCLUDE \epoc32\include \epoc32\include\stdapis` resolves to the **host** `/epoc32/...`. Absolute Symbian paths must be joined to `SYMDEV_EPOCROOT`. Every real MMP carries this line. | `driver/resource.rs: mmp_dir_path` | blocking |
| 7 | `.c` sources are compiled with `SYMDEV_GXX` (g++). The SDK compiles them as C; Tatham's code does not compile as C++ (`enum { GOT_SEED, … }` inside a struct, and more). | `driver/build.rs` | blocking for any ported C |
| 8 | The SDK's `epoc32/include/gcce/gcce.h` expands `va_start(ap,p)` to `__builtin_va_start(ap.__ap, p)`, which assumes `__builtin_va_list` is `void*`. In GCC 12 `arm-none-symbianelf` it is itself `struct __va_list { void *__ap; }`, so every varargs function fails to compile. symdev force-includes `gcce.h`, so this hits any project that uses `VA_LIST`/`va_list`. | `driver/compile.rs` | blocking |
| 9 | `aknicon.dso` declares its link name as `AknIcon{000a0000}.dso`; the lookup in `--libpath` is case-sensitive, so linking dies. symdev already case-folds SDK *headers* (`sdk-include-casefold`) but not libraries. | `symdev-elf2e32` | high |
| 10 | Generated headers are looked up case-sensitively: the sources `#include <puzzles_0xa000ef77.rsg>`, the build writes `Puzzles_0xa000ef77.rsg`. | `driver/build.rs` | medium |
| 11 | `symdev package` looks for `build/<package.name>.exe` while `symdev build` writes `build/<MMP TARGET>.exe`. The two disagree whenever the MMP target is not the manifest name. | `symdev-cli/src/main.rs` | high |
| 12 | No manifest field for extra install files. Puzzles ships a bitmap store, an icon MIF and two fonts; `SisPackage` also insists every file sit in `build/`. This run needed a throwaway binary over `BuildOutputs` + `SisPackage` to get a `.sisx`. | `symdev-manifest`, `package.rs` | high |
| 13 | The icon model is exactly one SVG installed as `\resource\apps\<app>_aif.mif`. Puzzles wants `\resource\apps\<uid>\puzzles.mif` plus a 34-bitmap `games.mbm`/`games.mif` pair, with a generated `.mbg` the sources include. | `icons.rs` | high |
| 14 | No BMP path at all: no `bmconv`, no `.mbm`, no `.mbg` generation. The spec is written ([bmconv-spec.md](bmconv-spec.md)); nothing implements it. | — | high |
| 15 | The native SVG encoder rejects the app's real icon: `TODO: attribute version on <svg> (not in the icon subset)`. `gfx/app.svg` is stock Illustrator SVG Tiny 1.1 (`version`, `baseProfile`, `id`, `x`/`y`, `xml:space`, `<path fill-rule>`). Honest refusal, but no real-world icon passes it. | `symdev-mif` | medium-high |

## Workarounds used in this run (all throwaway)

1. Rewrote `bld.inf` and `Puzzles.mmp` without comments, without the preprocessor and without the
   directives symdev does not know (gaps 1–5); kept only `PRJ_MMPFILES Puzzles.mmp`.
2. Wrote the host SDK paths into `SYSTEMINCLUDE` (gap 6).
3. Pointed `SYMDEV_GXX` at a shim that dispatches `.c` to `arm-none-symbianelf-gcc -std=gnu89`
   and drops the C++-only flags (gap 7).
4. Added `inc/symdev_compat.h`, included from `PuzzlesConfig.h` and `puzzles.h`: re-points the
   `va_*` macros at `*(__builtin_va_list *)&ap` (gap 8) and defines `COMBINED` / `STYLUS_BASED`
   (gap 3).
5. Renamed `data/Puzzles.rss` → `data/Puzzles_0xa000ef77.rss` (and the `_reg`) so the resource
   stem gives the names the sources expect (gap 4); fixed the `.rsg` include case (gap 10).
6. Symlinked `build/AknIcon{000a0000}.dso` → the SDK's lowercase file (gap 9).
7. Set `[package] name = "Puzzles_0xa000ef77"` to match the MMP target (gap 11).
8. Generated `games.mbm` with Wine `bmconv` and `puzzles.mif` with Wine `mifconv`
   (`/S<tools> /T<tmp>`, short paths — see [svgb-mif-spec.md](svgb-mif-spec.md) §146) (gaps 13–15).
   The `.mbg` came from `mifconv /H`; for `.bmp` input `mifconv` writes an almost-empty `.mif`
   plus a sibling `.mbm`, and its own `bmconv` call is broken under Wine
   (`\epoc32\tools\BMCONV.exe\bmconv …`), so `bmconv` was run directly.
9. Changed `_UID3` in `inc/Puzzles.hrh` from `0xA000EF77` to `0xE000EF77` so the registration
   resource matches a self-signable UID. Not a symdev gap — a protected-range UID cannot be
   self-signed.

`games.mbm` (34 bitmaps, 209 743 bytes), `games.mif`, `puzzles.mif` and the `.mbg` from this run
are usable as goldens for a native `bmconv`; they were produced under Wine from the repository's
own `gfx/*.bmp` and `gfx/app.svg`.

## Suggested order for closing the gaps

1. MMP/bld.inf front end: comments, preprocessor, the missing directives, `gnumakefile`,
   `START RESOURCE TARGET` (gaps 1–5). One slice, unblocks nearly every real project.
2. Path and case handling: EPOCROOT-relative absolute includes, case-folded `.dso` and build-dir
   lookups (gaps 6, 9, 10).
3. Compiler dialect: `.c` through the C compiler, and a `gcce.h` varargs shim (gaps 7, 8).
4. Packaging: one name for the EXE, extra install files in the manifest (gaps 11, 12).
5. Icons: `bmconv` from the existing spec, `.mbg`, a wider icon model, a wider SVG subset
   (gaps 13–15).
