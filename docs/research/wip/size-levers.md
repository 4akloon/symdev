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
