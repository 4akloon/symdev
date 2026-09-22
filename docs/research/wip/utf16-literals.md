# WIP: compile-time UTF-16 literals (experiment 106)

Task: measure which `symbian-rs/examples/*` (not `std-*`) could drop `Buf16::push_str`
(run-time UTF-8 -> UTF-16); if worth it, put compile-time-known text into the image as
UTF-16 (const fn behind `Lit16`, fast `write!` literal pieces, or `u16!`). `hello` is
1 245 B vs C++ 802 B. Output byte-identical; extend `crates/symdev-build/tests/fast_write*.rs`.
Measure every example before/after; emulator tests must pass. Record as experiment 106.
Mine: `symbian-core/src/des/**`, `symbian-macros/src/fast_write/**`. Not mine: symbian-ui,
shims/s60, macros/entry.rs, std/src/fs, core/src/fs.

## Findings

- Baseline (main de45e20, `~/.cache/utf16-literals-agent/res-base.txt`, exe bytes / UTF-16 symbol bytes): alloc 3767/656, async 18603/448, atomics 8903/640, cleanup 4472/448, files 9341/448, fmt 100031/3456, hello-raw 808/0, hello 1245/428, locale 8202/448, net 10640/448, notes 12805/520, panic 2010/432, query 14031/532, shim 4517/648, spawnee 3076/448, time 10256/448, tls 14114/736, ui-list 12624/520, ui 11645/520.
- Who links push_str and why: every report-writing example (result-file path via `fs::path_of` -> `Buf16<256>::push_str`), `panic` and `spawnee` (fs paths), `alloc` (`HBuf16::push_str` of a run-time `String`), `shim` (FileServer paths; its own literal `push_str`s into `Buf16<160>` are a separate 120-byte monomorph). Only `hello` links it purely for compile-time-known text; `hello-raw` already has none.
- Buf16 destinations of the fast `write!` in examples: only `hello`, `alloc` (its one Buf16 `write!` has `{over_byte:x}`, so it is `core::write!` whole) and `fmt` (test). The harness writes into `String`, so report examples are not touched by a Buf16-only literal path. `shim` calls `push_str("literal")` directly six times (Buf16<160> monomorph 120 B) but keeps the shared encoder for FileServer paths.
- `TDes16::Append(const TDesC16&)` is already bound (observed); `hello` imports `__aeabi_memclr4` from drtaeabi, so a units copy should also resolve to ROM (check).
- A proc macro cannot tell that `{GREETING}` names a `const`: constness is not in the type, so autoref specialisation cannot see it either, and a `const {}` block around a run-time variable is a compile error, not a fallback. So `{GREETING}` of a `&str` stays on `push_str`.

- Prototype (utf16! + fast write! literal pieces as UTF-16 into Buf16): hello 1245 -> 956 B, no UTF-16 symbol left; E32Main folds to inline stores of the greeting, AppendNum, and one __aeabi_memcpy (drtaeabi, ROM) of " chars)". Remaining vs hello-raw 808: Buf16::new's memclr of the unused buffer tail and the #[main] trap/cleanup entry (not mine: entry.rs).

- Measured all examples with the prototype: only alloc +4 (lost IPSCCP specialisation of Generic<String>::put_str for "," — fixed by #[inline(always)] on the default put_utf16 and Arg::put, back to 3767) and fmt 100031 -> 103311 (+3280: the unit copy is inlined at every literal piece into each of 26 Buf16<N>; ~+20 B per site against a push_str call). That per-site growth would hit any real program with many literal pieces into a Buf16.
- `TDes16::Append(const TUint16*, TInt)` is exported (`_ZN6TDes166AppendEPKti` in euser.dso). Probe compiled by symdev from a C++ project (scratchpad/appendprobe): `probe_append_ptr(d,p,n){d->Append(p,n);}` is `push {r4,lr}; blx Append; pop` — no shuffle, so this=r0, ptr=r1, len=r2, as exp 78's rule says.

- Where the unit copy lives (hello / fmt exe bytes; fmt has 703 literal pieces into 25 Buf16<N>): V0 inlined room check + euser `Append(ptr,len)` at each site 939 / 105905; V1 `#[inline(never)]` `Sink::put_utf16` per N 971 / 97666; V2 one non-generic helper per image 972 / 100454; earlier inlined Rust copy loop (memcpy) 956 / 103311; base push_str 1245 / 100031. Inlining costs ~12 B per literal piece; V1 costs one ~48 B function per capacity (break-even ~4 pieces per N).

- Identity: HostBuf now models Buf16 in UTF-16 units (cap in units, put_utf16 copies the macro's units, never the &str); all 16 existing fast_write tests pass on it; new crates/symdev-build/tests/fast_write_utf16.rs (5 tests) passes. Breaking the const encoder's low surrogate (0x3ff -> 0x1ff) fails all 5 new tests (the old 9 do not notice: no astral literal in them).

- Gates clean: cargo test --workspace, clippy --all-targets (host), clippy --release --workspace (symbian-rs, only the pre-existing compiler_builtins profile warning). symbian-macros 26 tests pass (expansion test updated for the utf16! constant).

- Final sizes (res-final.txt): hello 1245 -> 971, fmt 100031 -> 97666, every other example the same size; atomics, time, query, alloc .exe differ from main's only at offsets 20-23 and 36-39 (E32 header CRC and build time), so their code is byte-identical.

- Emulator, `symdev test --emulator` (run-final): net 22, async 15, atomics 23, cleanup 2, files 26, fmt 14, locale 7, notes 3, query 4, time 29, tls 45, ui 3, ui-list 6 passed — the counts of exp 103; shim writes no report (its note: "shim70 mkdirall=0 trapped=-12 bad=0 ensured=0 sign=-42 alive"). hello's note from build/eka2l1.log: "Trying to display: Hello from Rust SDK (19 chars)".
- examples/fmt gained case 13 "text known at compile time, appended as UTF-16" (astral literal pieces straddling every capacity, a utf16! constant as `{MIXED}` and `{}`): 15 passed. Deliberate break (append_units passes len-1 to euser): 12 failed, 3 passed (every group with literal text, incl. the new one with 48 mismatches). Reverted.
- C++ hello rebuilt from docs/research/cpp-parity/hello: 802 (E32Main 52 + KFormat 32 + KGreeting 44). Rust hello 971: E32Main 260 (CTrapCleanup pair of #[main], memclr4 of Buf16::new's [0; 64], a room check per piece), put_utf16<64> 48, rodata 56 (the UTF-16 text).

## Decisions

- Design to try: `symbian_fmt::Utf16Str` = `&'static str` + its `&'static [u16]`, built only by `utf16!(expr)` (const items: `[u16; utf16_len(S)]` from a const fn), `Deref<Target = str>` and `Display` = str's, `Arg` -> new `Sink::put_utf16(text, units)` whose default is `put_str(text)` (so every non-Buf16 destination sees the same `write_str`), `Buf16` overrides with a room check + unit copy. The fast `write!` emits each literal piece as a `utf16!` const. `hello` changes one line: `const GREETING: Utf16Str = utf16!("…")`.
- Chose V1: a literal piece costs its call site what a `push_str` call did, and one function per `Buf16<N>`; `hello` pays 32 B against V0 for it, and `fmt` (many sites) shrinks instead of growing 5.9 kB. The copy is euser's `TDes16::Append(const TUint16*, TInt)`, in ROM.

## Dead ends

## Next step

Measure every example (final), run symdev test --emulator on every report writer, read hello's notifier line, weigh shim.
