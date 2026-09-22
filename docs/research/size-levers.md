# Size levers on the `no_std` path (2026-09-21/22)

The user's priority: resource use at parity with the C++ version wherever possible, built
on `no_std`. This note lists every lever tried on the Rust side, one change at a time,
each measured across **all sixteen `no_std` examples** (`symbian-rs/examples/*` except
`std-hello` and `std-net`). A lever that shrinks one example and grows another is
reported as a trade, not a win. Branch `size-levers`, baseline `main` at 4c5fc5e.

**Method.** `symdev build` in every example directory; the number is the `.exe` byte size
(what ships: deflate-compressed E32 image) plus `arm-none-symbianelf-size -A` on the
`.elf` (`.text`, `.rodata`, `.ARM.exidx`). "Why" comes from the link map's "Archive member
included" and "Discarded input sections" and from `nm --size-sort`. Differences of ±4–16
bytes in `.exe` with **identical** section sizes are deflate noise from a changed layout,
and are called that. The C++ side is measured separately (`docs/research/cpp-parity*`).
Where the SDK's own build files answer "does C++ already get this?", the answer is
quoted from them. Nothing was built in C++ for this note.

## Result

Kept: L1, L2, L4, L9. Cumulative `.exe` bytes (each column adds one lever to the one before):

| example | baseline | +L1 | +L2 | +L4 | +L9 | total |
|---|---|---|---|---|---|---|
| `alloc` | 4 474 | 4 474 | 4 474 | 3 832 | 3 832 | -642 (-14 %) |
| `async` | 21 659 | 21 668 | 21 675 | 20 427 | 20 406 | -1 253 (-6 %) |
| `atomics` | 11 719 | 11 719 | 11 714 | 10 695 | 10 672 | -1 047 (-9 %) |
| `files` | 10 552 | 10 552 | 10 552 | 9 494 | 9 494 | -1 058 (-10 %) |
| `hello` | 3 187 | 3 187 | 3 187 | 2 523 | 2 523 | -664 (-21 %) |
| `hello-raw` | 752 | 752 | 752 | 752 | 752 | 0 |
| `locale` | 9 684 | 9 684 | 9 684 | 8 451 | 8 451 | -1 233 (-13 %) |
| `net` | 13 379 | 13 379 | 13 380 | 12 356 | 12 356 | -1 023 (-8 %) |
| `notes` | 15 626 | 15 630 | 15 630 | 14 648 | 14 648 | -978 (-6 %) |
| `query` | 20 449 | 20 460 | 20 461 | 19 372 | 19 372 | -1 077 (-5 %) |
| `shim` | 4 474 | 4 474 | 4 474 | 4 474 | 4 474 | 0 |
| `spawnee` | 3 208 | 3 208 | 3 208 | 3 208 | 3 208 | 0 |
| `time` | 20 583 | 20 583 | 14 791 | 13 531 | 13 531 | -7 052 (-34 %) |
| `tls` | 16 272 | 16 272 | 16 265 | 15 036 | 15 008 | -1 264 (-8 %) |
| `ui` | 13 714 | 13 703 | 13 703 | 12 958 | 12 958 | -756 (-6 %) |
| `ui-list` | 14 773 | 14 610 | 14 594 | 13 870 | 13 870 | -903 (-6 %) |

Corpus total: **`.exe` -18 950 bytes, `.text` -23 100 bytes.** No example grows by more than
the noise described above.

**The largest remaining gap was `core::fmt` (L8).** In `hello` it was 1 330 of the 2 523 bytes
that were left. The macro it needed was decided and built in experiment 101: `hello` is now
1 245 bytes with no `core::fmt` in it (see L8). Experiment 102 took it out of the test harness, and with it out of
every example that reports (see L8).

## Every lever

### L1 — `-ffunction-sections -fdata-sections` on symdev's own C++ shims — **KEEP (small)**

*Change:* `RustBuild::SHIM_SECTIONS` (`crates/symdev-build/src/driver/rust_shims.rs`), passed
through the `OPTION GCCE` slot of the recorded compile line. The C++ line for real C++
projects is untouched. **These two flags are not a reproduction of an SDK tool.** Every other
flag on the line is the SDK's own. These two are symdev's choice for its own C++
(`symbian-rs/shims/**`, which nothing in the SDK compiles), so the rule against inventing
argv does not apply to them. The code comment says so too.

*Numbers:* `.exe` -150, `.text` -236 across the corpus. `ui-list` -163 (`.text` -220, `.ARM.exidx`
-40), `async` `.text` -16. `notes` +4, `query` +11 and `ui` -11 have identical sections, so
those changes are deflate noise.

*Mechanism:* the collector now drops the **out-of-line copies of methods gcc already
inlined into their only caller**: `CSymRsList::NewL` 80, `SetSelected` 40, the constructor 32,
`Selected` 20, `Clear` 16 and `Count` 16, together with their unwind entries.

*The brief's hypothesis for experiment 95 was wrong.* Even with the flags, nothing in
`symrs_avkon.o` is collected. Its kept sections are `.rodata._ZTV10CShimAppUi` (344) and the
**virtual** methods that vtable names, including `DynInitMenuPaneL` (160) and `HostMenuItem`
(180). They are reachable through the vtable, so no section granularity can help. The
284–329 bytes of experiment 95 are what a vtable slot costs. All four GUI examples also
declare `softkeys`, so none of them is a GUI example without a menu.

*C++ default:* no. `epoc32/tools/compilation_config/gcce.mk` has
`REL_OPTIMISATION=-O2 -fno-unit-at-a-time` and no section flags, and the recorded link line
has no `--gc-sections` (symdev adds it for Rust only, in `rust_link.rs`). A C++ application
keeps every function of every object it links.

### L2 — one codegen unit per `compiler_builtins` builtin — **KEEP (big)**

*Change:* `[profile.release.package.compiler_builtins] codegen-units = 10000` in
`symbian-rs/Cargo.toml` and in the scaffold's `Cargo.toml` template
(`crates/symdev-cli/src/scaffold_rust.rs`).

*Numbers:* `time` goes from 20 583 to 14 791 `.exe` bytes (-5 792, -28 %) and `.text` shrinks
by 9 168. Every other example stays within ±16 bytes of noise.

*Mechanism:* the link map names `__aeabi_uidiv` as the only reference into
`libtimedemo.a(compiler_builtins-….cgu.0.rcgu.o)`, and that one object was 9 516 bytes:
`__divdf3` 1 056, `__adddf3` 908, `__muldf3` 836, `u64_div_rem` 636, `__divsf3` 588,
`__addsf3` 528, a second `memcpy` 432, and more. After the change `nm` finds exactly two
builtins in the image, `u32_div_rem` (212) and `__udivsi3` (16). The euser/drtaeabi-first
link order (experiment 77) was a workaround for this. It still matters for `mem*`, but it
could not help a division.

*C++ default:* **yes, already.** The C++ link ends in `-lgcc` (`driver/link.rs:104`), and
`libgcc.a` has 1 758 members, with `_udivsi3.o`, `divdf3.o` and `adddf3.o` as separate
members. One division pulls one routine.

### L3 — `opt-level = "z"` instead of `"s"` — **REJECT (a trade)**

The corpus total is `.exe` -1 849, but five of the sixteen examples grow: `alloc` +531, `ui-list`
+305, `notes` +184, `ui` +160, `hello` +132. Against that, `async` -1 052, `time` -477 and
`files` -419. The setting stays `"s"`. (The SDK compiles C++ at `-O2`, not `-Os`.)

### L4 — `-Zbuild-std-features=optimize_for_size` — **KEEP (big, uniform)**

*Change:* `RustBuild::BUILD_STD_FEATURES` on the `no_std` cargo line only, because naming the
flag replaces cargo's default feature set, which is `panic-unwind` for the `std` examples,
and those were not measured. The same flag goes on `LibcallArchive::cargo_args`, into
`symbian-rs/.cargo/config.toml` and into the scaffold's `.cargo/config.toml`.

*Numbers:* every example that formats anything loses 600–1 450 bytes of `.text` and
exactly 200 of `.rodata`, and none grows. `hello` goes from 3 187 to 2 523.

*Mechanism:* the 200 bytes are `core`'s two-digit `DEC_DIGITS_LUT`. The integer `Display`
becomes `display_u32_small` (`usize Display::fmt` 908 → 636), and `str Display::fmt` shrinks
from 784 to 400. The trade is fewer bytes for more cycles, and nothing observable changes.

*C++ default:* no counterpart is needed, because C++ formats through euser's
`TDes::AppendNum`/`Format` in ROM, so none of it lands in the image.

### L5 — `-Zlocation-detail=none` — **REJECT (zero)**

`cargo build -v` confirms the flag reaches rustc, and it changes **0 bytes** in all sixteen
examples. `symbian-runtime`'s `#[panic_handler]` ignores its `&PanicInfo`, so under
`panic = "abort"` and LTO the `Location` statics are already dead.

### L6 — `-Zfmt-debug=none` — **REJECT (large number, real loss)**

The corpus total is `.exe` -19 427, which is larger than the kept levers together. It is
rejected because it blanks `{:?}` output that is actually used. `symbian_std::test_report::
Report::checked` writes the failure detail with `write!(detail, "{e:?}")`, so every failing
`symdev test` case would report an empty reason. `async`, `query`, `time`, `notes` and `tls`
also format `{:?}` themselves. Most of what it removes is the `KErr*` name table (see "Not a
lever" below).

### L7 — `-Cpanic=immediate-abort` — **a trade, left unapplied**

The corpus total is `.exe` -2 438 and `.text` -3 364, and no example grows. It removes the
`core::panicking` stubs and the `Arguments` that each panic site builds for a message nobody
reads. *The trade:* a Rust panic then traps instead of reaching `#[panic_handler]`, so it
no longer exits through `User::Exit(-1)`, `symdev test` sees a kernel fault instead of an
exit code, and the intended `User::Panic` with a category could not be reached. That handler
is in `symbian-runtime`, which another agent owns, so the decision is recorded here and not
made here.

### L8 — `core::fmt` behind `write!` — **applied in experiment 101 (`symbian_std::write!`)**

*Achieved (experiment 101, `main` at aae58bb):* `symbian_std::{write, writeln}` — `write!`'s
syntax, byte-identical output (tested call for call on the host and unit for unit on
`Buf16` in the emulator), plain `{}` of strings and integers appended directly, anything
else `core::write!`. `hello` **2 567 → 1 245** (C++ 802; the hand-written ceiling below was
1 193) with **no `core::fmt` symbol left**. It is not in the prelude: a glob-imported
`write` is E0659-ambiguous with `core`'s, so a program opts in with
`use symbian_std::{write, writeln};`. It saves only where nothing else needs `core::fmt`:
`alloc` −128, but `async` +548, `locale` +230, `query` +161, `time` +880, because the test
harness (`check_detail(fmt::Arguments)`, `checked`'s `{e:?}`, the JSON writer's `{:08x}`)
keeps `core::fmt` in every example that reports, and is **all** of it in eight of them.
*Harness (experiment 102):* `check_detail` now takes `detail!(…)` (the fast `write!` in a
closure), `checked` records an error through `test_report::Evidence` (`KErrNotFound (-1)`),
and the JSON is plain appends. `core::fmt` is gone from every example that reports;
`async` −1 848, `atomics` −1 794, `cleanup` −1 318, `files` −1 726, `locale` −1 812, `net`
−1 787, `notes` −1 866, `query` −5 364, `time` −3 305, `tls` −992, `ui` −1 322, `ui-list`
−1 283, none grows. With it the fast `write!` shrinks the four it had grown (`async`
−1 116, `locale` −1 351, `query` −1 494, `time` −1 335 against `core::write!` over the same
harness), and all four use it. Only `alloc` (its own `{:x}`) and `fmt` (a comparison with
`core::write!`, on purpose) still link `core::fmt`. What is left of `hello`'s gap is
`Buf16::push_str` (428, UTF-8 to UTF-16 at run time); compile-time UTF-16 literals are the
next lever.

*The measurement that motivated it (before experiment 101):*

In `examples/hello` I replaced the single `write!` by the four calls a macro would generate
(`push_str`, `push_str`, `append_num`, `push_str`) and changed nothing else:

| `hello` | `.exe` | `.text` |
|---|---|---|
| `write!` (after L1+L2+L4) | 2 523 | 2 908 |
| hand-expanded | **1 193** | **832** |

That is -1 330 bytes (-53 %), or -62 % from the 3 187-byte baseline. The C++ `hello` recorded
in this project is 746 bytes.

*What `write!` actually brings in* (`nm`, after L4): `usize Display::fmt` 636,
`core::fmt::write` 536, `str Display::fmt` 400, `Buf16`'s `fmt::Write::write_char` 192,
`Formatter::padding` 192, `pad_integral::write_prefix` 100. The integer conversion is not
the cost: `Buf16::append_num` already sends the digits to euser's `TDes16::AppendNum`, which
adds zero bytes. The cost is building the `Arguments`, the `Formatter` width and precision
handling, and the `fmt::Write` shim that every piece of a `write!` goes through.

*Why it was not applied then:* using it means adding a public formatting macro, and that is a
decision about developer experience. The shape that costs nothing is a proc macro with
`write!`'s exact syntax. It would emit direct pushes for `{}` and `{name}` over `&str`,
integers and `char`, and **fall back to `write!` for any piece it cannot handle natively**
(`{:>8}`, `{:x}`, a user `Display`). No program would lose an ability. Only the programs
that never needed `core::fmt` would stop carrying it. What remains in the 832 bytes is
`Buf16::push_str` (432, UTF-8 to UTF-16) and about 350 bytes of fixed SDK entry code
(`_E32Startup`, `CallThrdProcEntry`, `__cpp_initialize__aeabi_`). If the macro also emitted
literals as compile-time `&[u16]` descriptors, most of the 432 would go as well.

### L9 — `-Zdefault-visibility=hidden` for the libcalls archive — **KEEP (tiny)**

Applied to the whole workspace it saves `.exe` -72, all of it 24 bytes of `.text` in each of
`async`, `atomics` and `tls`. A `nm` diff shows that the only symbol removed is
`<symbian_libcalls::lock::AtomicLock as Drop>::drop`. It is an out-of-line copy of a guard
that is inlined into every `__atomic_*` entry point. The copy survives only because the
non-LTO libcalls rlib exports it globally, and every global symbol is a `--gc-sections`
root in a `-shared` link. The flag is applied on the libcalls cargo line only
(`--config build.rustflags=["-Zdefault-visibility=hidden"]`), and re-measuring gives the same
-72. The `no_mangle` entry points stay `GLOBAL DEFAULT`, as experiment 80 found. *C++:* no
counterpart (`RUNTIME_SYMBOL_VISIBILITY_OPTION=` is empty in `gcce.mk`).

### Not a lever: the `KErr*` name table

About 1.2 kB of `.rodata` (`KErrNotSupported…`) sits in every example that calls
`Report::checked`, which reaches it through `ErrorKind::name()` (since experiment 102 from
`SymbianError`'s `Evidence`, before that from its `Debug`): the name is how a failed case
identifies the error. That
is the **test harness**. `hello` does not use `Report` and carries none of it, so a shipped
application does not pay for it.

### Not a lever: `.bss` / `.data`

`.data` is 0 everywhere except `async` (40 bytes). `.bss` ranges from 0 to 72 bytes
(`atomics` 72, `tls` 52, `async` 40, most examples 24, `hello` 0). There is nothing to take.

### Considered, not tried

- *`strip`*: `elf2e32` copies only the loaded sections and the import table into the image,
  so the `.comment`, symbol and string tables of the `.elf` never ship.
- *Byte-pair E32 compression*: the image is already deflate-compressed by default. Byte-pair
  is the faster-loading method, not the smaller one, and no argv for it has been observed.
- *`debug-assertions` / `overflow-checks`*: both are already off by default in `release`, and
  the profile does not turn them on.

## Behaviour

Host gates: `cargo test --workspace --offline` (with `SYMDEV_SIGN_PASSWORD` unset) and
`cargo clippy --workspace --all-targets --offline` both pass with no warnings. The two argv
tests now pin the new lines: `-Zbuild-std-features=optimize_for_size` in the cargo
invocation, and the two section flags after `-mapcs` on the shim compile.

`symdev test --emulator` with every kept lever applied:

| example | result |
|---|---|
| `async` | 15 passed |
| `atomics` | 23 passed |
| `files` | 16 passed |
| `locale` | 8 passed |
| `notes` | 3 passed |
| `query` | 4 passed |
| `time` | 29 passed |
| `tls` | 45 passed |
| `ui` | 3 passed |
| `ui-list` | 6 passed |
| `net`, `shim` | **no result file after 180 s — the same on the untouched baseline** |

`net` and `shim` fail identically when built from `main` at 4c5fc5e in a separate worktree,
with byte-identical baseline images (13 379 and 4 474). The failure predates this work, and
this audit did not diagnose it. `shim`'s image is not changed by any kept lever, and `net`'s
changes only through L4.

Driven by hand (`emukey.py`, 900×600, N00):

- `ui`: F1 opens Options with Avkon's "Show open apps." above More bars / Fewer bars / Reset /
  Exit. Down and Return on "Fewer bars" show **`bars=2 keys=0 cmd=1`**, which is experiment 95's
  result, rendered by the smaller `Display` from L4.
- `ui-list`: Down ×3 and Return on "Charlie" rewrite the first row to **`picked: 3`**. The shim
  methods that L1 lets the linker drop are really unused.

Not run: `hello`, `hello-raw`, `alloc`, `spawnee`. They write no report and were not driven.
`hello-raw` and `spawnee` are byte-identical to the baseline. `hello` and `alloc` change only
through L4, the same `core` code that `ui`, `locale`, `time` and the others run above.

## Note on `Debug` (data point from `nostd-readdir`, another agent's branch)

In those images `{:?}` is a separate cost from `Display`, and a larger one.
`format_args!("{e:?}")` on a `symbian_std::io::Error` cost `examples/cleanup` **1 547** bytes
(8 015 → 6 468 when replaced by an `{}` of the raw code). Removing a
`{:?} {:?}` of `e.kind()` and `e.raw_os_error()` took `examples/files` from 12 986 to 12 069
(-917). The added symbols are `PadAdapter::write_str` +596, `<Option<i32> as Debug>::fmt`
+348, `<i32 as Debug>::fmt` +160 and `PadAdapter::write_char` +104. `PadAdapter` comes with
any `debug_tuple`/`debug_struct`/`Option` shape, because it implements the `{:#?}`
indentation. A plain integer `{}` added nothing, because `Report` already linked it. So
L6's -19 427 is mostly this, plus the `KErr*` table. The cheap habit that follows: SDK
examples and docs should write an error as `{}` of its code or name, not as `{:?}`. Since experiment 102 the
examples show errors and outcomes through `test_report::Evidence` (`.shown()`), which
writes `Debug`'s text without `core::fmt`.

## Harness

The measurement scripts are kept outside the repo (`~/.cache/size-levers-agent/measure.sh`
and `diff.sh`). For each example they run `symdev build`, record `stat -c%s build/*.exe` and
the `size -A` rows `.text`, `.rodata`, `.ARM.exidx`, `.ARM.extab`, `.constdata`, `.data` and
`.bss`, and then diff two runs line by line. After the build cache is warm, a full
sixteen-example run takes about 15 s. A cold run takes about 4 minutes.
