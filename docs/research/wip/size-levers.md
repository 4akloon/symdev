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

### L2 — one codegen unit per `compiler_builtins` builtin (KEEP, big)

`[profile.release.package.compiler_builtins] codegen-units = 10000` in
`symbian-rs/Cargo.toml` and in the scaffold's template
(`crates/symdev-cli/src/scaffold_rust.rs`).

**`time`: exe 20 583 -> 14 791 (-5 792, -28 %), text 27 972 -> 18 804 (-9 168).**
Every other example unchanged (±16 B of deflate noise).

Mechanism: `build/timedemo.exe.map`'s "Archive member included to satisfy reference"
names `__aeabi_uidiv` as the one reference into
`libtimedemo.a(compiler_builtins-….cgu.0.rcgu.o)`, and that single object was 9 516 B:
`__divdf3` 1 056, `__adddf3` 908, `__muldf3` 836, `u64_div_rem` 636, `__divsf3` 588,
`__addsf3` 528, a second `memcpy` 432, `__mulsf3` 412, `__truncdfsf2` 324 … After the
split, `nm` finds exactly two builtins left in the image: `u32_div_rem` (212) and
`__udivsi3` (16) — 228 B instead of 9 516.

### L3 — `opt-level = "z"` instead of `"s"` (REJECT, a trade)

`symbian-rs/Cargo.toml`. Corpus total exe -1 849, but it is not a win everywhere:
`alloc` +531, `ui-list` +305, `notes` +184, `ui` +160, `hello` +132 against
`async` -1 052, `time` -477, `files` -419. A lever that grows five of sixteen is a
trade; left at `"s"`.

### L4 — `-Zbuild-std-features=optimize_for_size` (KEEP, big and uniform)

`RustBuild::BUILD_STD_FEATURES` (`no_std` path only, because naming the flag replaces
cargo's default feature set and the `std` examples' default is `panic-unwind`), the
same line in `LibcallArchive::cargo_args`, in `symbian-rs/.cargo/config.toml` and in
the scaffold's `.cargo/config.toml`.

Every example that formats anything drops **600–1 450 B of `.text` and exactly 200 B of
`.rodata`**; **none grows**. `hello` 3 187 -> 2 523 exe. The 200 B is `core`'s
`DEC_DIGITS_LUT` two-digit table; the text is the small integer `Display`
(`display_u32_small`), `usize Display::fmt` 908 -> 636, `str Display::fmt` 784 -> 400.

**Cumulative L1+L2+L4 against the 4c5fc5e baseline: exe -18 878, text -23 028.**
`hello` -21 %, `time` -34 %.

### L5 — `-Zlocation-detail=none` (REJECT, zero)

Verified the flag reaches rustc (`cargo build -v` prints it). **Zero bytes** on all
sixteen. `#[panic_handler]` in `symbian-runtime` ignores its `&PanicInfo`, so with
`panic = "abort"` and LTO the `Location` statics are already dead before this flag
looks at them.

### L6 — `-Zfmt-debug=none` (REJECT, the largest number in the audit, and a real loss)

Corpus total **exe -19 427, text -17 536** — bigger than everything else together.
It is rejected because it blanks `{:?}` output that is used: `Report::checked` in
`symbian-std` writes the failure detail with `write!(detail, "{e:?}")`, so every
`symdev test` failure would report an empty reason, and `async`/`query`/`time`/`notes`/
`tls` format `{:?}` themselves. What it actually deletes is visible in `strings`:
`SymbianError: Debug` prints `ErrorKind::name()`, and the whole `KErrNotSupported…`
table (~1.2 kB of `.rodata`) is in every example that calls `Report::checked`.

### L7 — `-Cpanic=immediate-abort` (a trade, left unapplied)

Corpus total **exe -2 438, text -3 364**, and no example grows. It deletes the
`core::panicking` stubs and the `Arguments` each panic site builds for a message
nobody reads. The trade, in one sentence: a Rust panic then traps instead of reaching
`#[panic_handler]`, so it no longer leaves through `User::Exit(-1)` and `symdev test`
would see a kernel fault rather than an exit code — and the design note's intended end
state is a `User::Panic` with a category, which this flag would put out of reach.

### L8 — `core::fmt` in `hello`: the measured ceiling (biggest lever, unapplied)

Measured, not argued. `examples/hello/src/main.rs`'s one `write!` replaced by the four
calls a macro would generate — `push_str`, `push_str`, `append_num`, `push_str` —
nothing else touched, same build:

| | exe | text |
|---|---|---|
| `write!` (with L1+L2+L4) | 2 523 | 2 908 |
| hand-expanded | **1 193** | **832** |
| delta | **-1 330 (-53 %)** | **-2 076 (-71 %)** |

Against the 4c5fc5e baseline that is 3 187 -> 1 193, **-62 %**, and the C++ `hello`
this project records is 746 bytes.

What `write!` actually referenced, from `nm --size-sort` (after L4):
`usize Display::fmt` 636, `core::fmt::write` 536, `str Display::fmt` 400,
`Buf16 as fmt::Write::write_char` 192, `Formatter::padding` 192,
`pad_integral::write_prefix` 100. So it is **not** integer formatting as such —
`Buf16::append_num` already hands the digits to euser's `TDes16::AppendNum`, which
costs zero bytes — it is `Arguments` construction, the `Formatter` width/precision
machinery and the `fmt::Write` shim that `write!` forces every piece through.

What is left in the 832 bytes: `Buf16::push_str` 432 (UTF-8 -> UTF-16) and ~350 of
fixed SDK entry glue (`_E32Startup`, `CallThrdProcEntry`, `__cpp_initialize__aeabi_`).
A macro that also emitted the literal as a compile-time `&[u16]` and appended it as a
descriptor would take most of the remaining 432 too.

**Left unapplied**: taking it means a new public formatting macro, and that is a DX
decision, not a flag. The shape that costs nothing: a proc macro with `write!`'s exact
syntax that emits direct pushes for `{}`/`{name}` over `&str`, integers and `char`,
and **falls back to `write!` for any piece it cannot do natively** (`{:>8}`, `{:x}`, a
user `Display`), so no program loses an ability — it only keeps `core::fmt` out of the
programs that never needed it.

### Not a lever: the `KErr*` name table

`strings` shows ~1.2 kB of `.rodata` of `KErrNotSupported…` in most examples. It is
reached only through `SymbianError: Debug` from `symbian_std::test_report::checked`,
i.e. from the **test harness**. `hello`, which does not use `Report`, carries none of
it (`-Zfmt-debug=none` moved it by 0 bytes). A shipped application does not pay it.

### Not a lever: `.bss` / `.data`

Measured at the baseline: `.data` is 0 everywhere except `async` (40 B), `.bss` is 0–72
(`atomics` 72, `tls` 52, `async` 40, most 24, `hello` 0). Nothing to take.
