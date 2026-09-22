# WIP: experiment 104 — smallest Avkon glue

Task: shrink the Avkon glue in `symbian-rs/crates/symbian-ui` (generic over the app type,
monomorphised per app: `vtbl::construct<Bars>` 2 212 B, `draw<Bars>` 1 360 B) plus the C++ shim
`symbian-rs/shims/s60/symrs_avkon.cpp` (~1.8 kB), keeping the public API unchanged
(`symbian_std::ui::App`, `#[symbian_std::main(gui)]`, `App::menu` closures, `List`, notes, queries).
Baseline: `examples/ui` 11 645 B vs C++ 7 317 B. Measure every non-std example before/after by symbol.
Verify: `symdev test --emulator` for every GUI example; ui by hand (F1, Down Return → `bars=2 keys=0 cmd=1`);
ui-list (Down Down Down Return → `picked: 3`). Out of bounds: symbian-std/src/fs, symbian-core/src/fs,
symbian-core/src/des, symbian-macros/src/fast_write.

## Findings

- Baseline (main de45e20, release symdev, `.exe` bytes): alloc 3767, async 18603, atomics 8903, cleanup 4472, files 9341, fmt 100031, hello-raw 808, hello 1245, locale 8202, net 10643, notes 12805, panic 2010, query 14031, shim 4517, spawnee 3076, time 10256, tls 14099, ui-list 12624, ui 11645. Scripts and nm dumps: `~/.cache/avkon-glue-agent/` (`measure.sh`, `res-base.txt`, `nm-base/`).
- ui symbols now: `construct<Bars>` 2468, `draw<Bars>` 1396, `offer_key<Bars>` 184, `command<Bars>` 140, `menu<Bars>` 92, `Menu<Bars>::item` 160, `menu::encode` 252, `encode_utf16_into` 240, `slice_error_fail_rt` 428.
- `draw<Bars>` is mostly NOT glue: `Bars::draw` is inlined into it, and so is `Gc::text` (UTF-8 decode + UTF-16 encode + the truncation fallback, ~830 B from 0x8264 to 0x85a0). Likewise `construct<Bars>` holds the inlined `Bars::construct` (the report). The audit's 2 212 / 1 360 are app code + inlined non-generic helpers, not generic glue per se.
- Probe (throwaway `examples/glueprobe`, empty App: `draw` = one `clear`, one menu item): the pure generic Rust glue is small — thunks size_changed 12, destroy 48, offer_key 48, construct 60, draw 64, create 64, menu 92, command 140, VTBL 36 = 564 B, plus `menu::encode` 252. Probe `.exe` 6 998.
- **One `App` type per image**, so monomorphisation duplicates nothing: a type-erased body would be the same bytes plus a dyn vtable. The per-type framing of the audit is not the mechanism; inlining is (`Bars::construct`+`Report::finish` into `construct<Bars>`, `Bars::draw`+`Gc::text` into `draw<Bars>`).
- ui buckets (nm sizes, 16 050 total): vtbl thunks incl. inlined app 4 360, EH runtime 3 776, **C++ shim 3 083**, test_report 1 200, alloc 956, symbian_ui other 752, symbian_core 616, core 488. C++ `cppui` whole app classes 2 795 (+ report 1 616), total 8 628. The shim alone is larger than the whole C++ app.
- Shim fat: `~CShimAppUi` three copies 152+152+160 (C++: 88×2), `ConstructL` 244 (C++ 46), `Draw` 120, `SizeChanged` 108, `HandleCommandL` 104 — every call site inlines `Vtbl()` (null + size check + `User::Panic(_L(...))`).
- Step 1, `utf16::encode_cut` (menu's cutting encoder, generalised to `&mut [u16]`, `#[inline(never)]`) now also behind `Gc::text`, replacing `encode_utf16_into` + the `char_indices().nth(64)` fallback: ui 11 645 → 11 298 (−347; `encode_utf16_into` 240 and `slice_error_fail_rt` 428 gone from ui, `draw<Bars>` 1 396 → 1 172), notes −164, query −202, ui-list ±0, probe −45. Behaviour: an overlong text now keeps as many whole chars as fit in 128 units (was: the first 64 chars), which is what the doc of `MAX_TEXT` already said.


## Dead ends

## Next step

Measure baseline sizes of every example.
