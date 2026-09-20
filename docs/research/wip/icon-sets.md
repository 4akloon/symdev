# icon-sets (branch `icon-sets`)

Task: let a real Carbide project declare its `mifconv` icon containers in `symdev.toml` instead of
`bld.inf` makefiles; `gnumakefile` lines become a warning; goal is
`/tmp/claude-1000/acceptance/run.sh build|package` on Puzzles with one added `symdev.toml`.

## Findings

All under Wine from the short cwd `/tmp/claude-1000/icon-sets-work/mif` with relative names
(svgb-mif-spec.md §3.1 trap 3). Tools: SDK `mifconv.exe` 1.11 build 49, `bmconv.exe` 112.

- `mifconv` needs `/S<dir>` even for bitmap-only input; without it: `ERROR: Binary converter
  'SVGTBINENCODE.exe' not found`. With `/S` and `/B<tools>` the whole bitmap chain runs under Wine
  (the "broken bmconv call" in third-party-app-puzzles.md was the missing `/B`).
- For `.bmp` sources mifconv writes a command file `<out>.mbm_###_bmconv_tmp_cmd_file` holding
  `/q <out>.mbm /c24blackbox.bmp  /c24bridges.bmp  /c24cube.bmp  ` and runs `bmconv <cmdfile>`.
  No `/h`: the `.mbg` is mifconv's own MIF-style header. The resulting `.mbm` is byte-equal to
  `bmconv /q out.mbm /c24a.bmp /c24b.bmp /c24c.bmp` run directly (3-bitmap case, and the 34-bitmap
  Puzzles set at 209 743 bytes).
- The bitmap `.mif` is a stub: `B##4`, v2, table@16, 2×N entries, each bitmap's two entries are
  `(offset = -(index in the .mbm) as u32, length 0)`. Depth-independent (c8/c24/1 mix gives the
  same 64 bytes as c24×3). 1 bitmap → 32 bytes, 3 → 64, 34 → 560.
- Bitmap mask `/c24,8`: command file gets `/c24blackbox.bmp /8blackbox_mask_soft.bmp`; the mask
  takes its own index slot in the `.mbm`, so entries are icon `(0,0)`, mask `(-1,0)`, next bitmap
  `(-2,0)`,`(-2,0)`. Missing mask file: `ERROR: EGray256 Mask not found! blackbox_mask_soft.bmp`.
- Bitmap mask `/c24,1` with both `_mask.bmp` and `_mask_soft.bmp` present still used
  `/8<stem>_mask_soft.bmp` — the `_mask.bmp`/`/1` path was never seen → refuse mask 1 on bitmaps.
- `.mbg` values always step by 2 per icon; the `_mask` line appears only when a mask depth was
  given (two SVGs at `/c32`: `A = 16384`, `B = 16386`). Same rule for bitmaps.
- Output named `x.mbm` with an SVG source writes `x.mif` (Puzzles' `Icons_scalable_dc.mk` names
  `puzzles.mbm`, the `.rss`/`.pkg` use `puzzles.mif`). A `.bmp` source into `g.mbm` writes both
  `g.mif` (32 B) and `g.mbm`. So the extension is always forced: `.mif`, plus `.mbm` sibling.
- Mixed `/c24 a.bmp /c32,8 x.svg /c24 c.bmp`: entries bmp `(0,0)×2`, svg block at 0x40 (=16+8×6)
  twice, bmp `(-1,0)×2`; `.mbm` equals direct bmconv over the bitmaps in order; `.mbg` interleaved
  in source order (Blackbox 16384, A 16386, A_mask 16387, Cube 16388).
- Goldens for the acceptance comparison: `/tmp/claude-1000/icon-sets-work/mif/golden/`
  (`games.mif` 560, `games.mbm` 209743, `puzzles_0xa000ef77.mbg` 1489, `puzzles.mif` 5658 —
  equal to the earlier session's `out_a/puzzles.mif`).

## Decisions

- TOML: `[[icons]]` = one mifconv call. `dest` (install path of the `.mif`, normalised like
  `[[install]].dest`, must end in `.mif`), optional `header` (bare `.mbg` file name, written to
  `build/`), optional `depth` (default for every source), `sources` = list of `"path"` or
  `{ file, depth, animated }`. Build files: `build/<dest stem>.mif`, `build/<stem>.mbm` when a
  source is a `.bmp`, installed next to the `.mif`. `[symbian] icon` unchanged.
- symdev-build depends on symdev-manifest so `IconContainer` is the domain type (no mirror).
- Refused by name: bitmap mask 1, `animated` on a bitmap, a depth with no source default.

## Dead ends

- `/B` alone (without `/S`) never reaches the bmconv stage — the usage text is printed after
  "Choosing...", no ERROR line is visible without `head`.

## Next step

Implement: bld warning → symdev-mif depth/bitmap/mask-less mbg → manifest `[[icons]]` → build
driver + package → acceptance loop → docs (backlog 64, gap table) → delete this file.
