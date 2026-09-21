# WIP: size levers for the `no_std` path

Task: find and measure every binary-size lever on the Rust `no_std` side (parity with C++),
one change at a time, against a baseline of every `symbian-rs/examples/*` except `std-*`.
Deliverable: `docs/research/size-levers.md`.

## Findings

- Baseline recorded 2026-09-21 on branch `size-levers` at 4c5fc5e, `symdev build` per example,
  sizes are `.exe` bytes and `arm-none-symbianelf-size -A` sections of the `.elf`:

```
name             exe    text rodata  cdata    emb   data    bss
alloc           4474    5680    326     16    200      0     12
async          21659   33512   2920     16    200     40     40
atomics        11719   15716   2596     16    200      0     72
files          10552   13436   2240     16    200      0     24
hello           3187    3524    268     16    200      0      0
hello-raw        752     220     44     16    200      0      0
locale          9684   11396   2388     16    200      0     24
net            13379   17696   2476     16    200      0     36
notes          15626   19920   2092     16    200      0     24
query          20449   23468   6080     16    200      0     24
shim            4474    5680    128     16    200      0      0
spawnee         3208    3956    555     16    200      0     24
time           20583   27972   3188     16    200      0     24
tls            16272   22016   4380     16    200      0     52
ui             13714   17192   1892     16    200      0     24
ui-list        14773   17600   2292     16    200      0     24
```
- `.bss`/`.data` are already negligible (max 72 B bss in `atomics`, 40 B data in `async`):
  lever 6 has nothing to take.
- Profiles are already tuned in `symbian-rs/Cargo.toml`: opt-level="s", lto=true,
  codegen-units=1, panic="abort", debug=false; a separate `libcalls` profile with
  lto=false, codegen-units=16.

## Decisions

## Dead ends

## Next step

- Read the hello/ui link maps and attribute the bytes.

### L1 — `-ffunction-sections -fdata-sections` on our own C++ shims (KEEP, small)

`crates/symdev-build/src/driver/rust_shims.rs`, `RustBuild::SHIM_SECTIONS`, fed through
the `OPTION GCCE` slot so the recorded C++ line for real C++ projects is untouched.

Result across the corpus: **exe -150 B, text -236 B**. All of it is `ui-list`
(exe -163, text -220, exidx -40) plus `async` (text -16). `notes` +4, `query` +11,
`ui` -11 with *identical* section sizes — deflate noise from a changed emission order.

Mechanism, read out of `build/*.map`'s "Discarded input sections": what `--gc-sections`
now drops from `symrs_list.o` is the **out-of-line copies of methods gcc already inlined
into their only caller** — `CSymRsList::NewL` 80, `SetSelected` 40, the ctor 32,
`Selected` 20, `Clear` 16, `Count` 16, plus their `.ARM.exidx`/`.extab`.

**The brief's hypothesis for experiment 95 is wrong.** Nothing of `symrs_avkon.o` is
collected even with the flags: the kept sections are `.rodata._ZTV10CShimAppUi` (344)
and the *virtual* methods it names, `DynInitMenuPaneL` (160) and `HostMenuItem` (180)
among them. They are reachable **through the vtable**, so section granularity cannot
help; the 284–329 B of experiment 95 is the vtable's fault, not the object's.
Also: all four GUI examples declare `softkeys`, so none of them is a "GUI example with
no menu" anyway.
