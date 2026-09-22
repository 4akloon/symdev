# fast-write

Task: a `write!`-compatible macro in `symbian_std::prelude` that turns plain `{}` of strings/integers into direct appends and falls back to `core::write!` otherwise, byte-identical output; measure against C++.

## Findings

- Baseline sizes on main 7231c84 (exe/text/rodata) in ~/.cache/fast-write-agent/res-base.keep; hello 2567. Measure script: ~/.cache/fast-write-agent/measure.sh <label>.
- Most examples reach core::fmt through `format_args!` into `Report::check_detail`, not `write!`. Direct `write!` users: hello (Buf16), alloc, locale/async/time (String `notes`), query (String), test_report itself (String).
- core's integer Display writes the sign with write_char then digits with write_str, so on an overflowing sink `-` can land alone: a fast path must reproduce that partial write.
- Buf16::append_num's `decimal_len` divides u64 by 10 at run time -> possible `__aeabi_uldivmod`; check.
- Autoref specialisation prototype (scratchpad/proto/p.rs) resolves as needed: concrete Buf-like sink -> Sink tier, String/Formatter/generic W: fmt::Write -> fmt::Write tier, custom Display and io::Write (Vec<u8>) -> slow closure; two-phase borrow ok for write!(b, "{}", b.len()); in a generic fn only the bound's tier is seen (correct, never wrong).
- rustc (1.98.1 and nightly-2026-09-19 identical) inlines literal args of a plain `{}` into the template: string literals (raw too), integer literals that fit their type (unsuffixed = i32; `256u8` under allow is not inlined), parenthesised `(5)` too; NOT char, bool, float, `-1`. Everything static -> one write_str even when empty. A macro that wants identical write_str call sequences must copy that rule (probe: scratchpad/proto/inl.rs).
- core writes: literal pieces via write_str; `{}` str -> write_str (pad fast path); char -> write_char; ints -> write_char('-') then write_str(digits). So identity is testable as identical *call sequences* on a recording fmt::Write, which implies identical bytes under any failing sink.
- symbian-rs cannot run host integration tests (build-std forced by config; `--target host --config unstable.build-std=[]` still duplicates core). Host tests live in the host workspace: `crates/symdev-build/tests/fast_write*.rs` with a path dev-dependency on `symbian-rs/crates/symbian-fmt`; it builds on stable 1.98.1 and the smoke test passes.
- `$dst` re-emitted as a bare invisible group lost its precedence: `write!(&mut h.line, ..)` became `&mut (h.line.method(..))` (E0716). Fixed by emitting `(@dst).__symbian_fmt_enter(..)`; parentheses keep a place expression a place.
- Host identity tests pass (16): call sequences equal under 56 failure modes each, plus Formatter/generic/io/eval-order/two-phase cases.
- DEAD END (conflicts with the user's «у prelude»): a macro named `write` in a glob-imported prelude does NOT shadow core's `write!`: rustc E0659 "`write` is ambiguous ... conflict between a name from a glob import and an outer scope during import or macro resolution" (nightly-2026-09-19 on hello; stable 1.98.1 in scratchpad/proto/mu: glob -> E0659, explicit `use dep::prelude::write` -> works, `#[macro_use] extern crate dep` -> works). Worse: exporting it from the prelude would BREAK every existing `use symbian_std::prelude::*` program that calls `write!`.

- hello: 2567 -> 1245 (.text 2972 -> 884): no core::fmt symbol left, not even panic_fmt; E32Main is 3 push_str calls + one AppendNum with the room check folded (ceiling was 1193).
- First cut grew every harness example (core::fmt stays because of Report::check_detail(format_args!) and {e:?}): alloc +273, async +733, locale +606, query +226, time +1035. Causes found by nm diff: core::str::from_utf8 572 B in Decimal::as_str (fixed with from_utf8_unchecked), 64-bit limb division 320 B used for every width (split Sink into put_u32/put_i64/put_u64), String::push_str inlined per piece (Generic methods #[inline(never)]).
- After fixes (res-fast3): alloc 3876->3748, async 20422->20971, locale 8486->8670, query 19372->19532, time 13523->14406, hello 1245; others unchanged. Remaining growth = per-piece call (~24 B each vs one Arguments build) + put_u32 188 + prepend_u32 88 + 64-bit path (of_u64 312, put_i64 164) when core::fmt is linked anyway.
- examples/fmt (uid 0xe00006a2) on the emulator: all 12 identity cases pass at 26 Buf16 capacities (0..=24, 64); the test can fail: dropping the lone '-' rule in Buf16's put_i64 gave 5 FAILs (e.g. 'negative numbers cut after the sign: 6 mismatches'). Ticks for 100 000 writes of "i={} neg={} s={}" (EKA2L1 1 ms NanoTicks): Buf16 core 65 / fast 55, String core 39 / fast 35.
- Harness share (res-fast3-{A,B,noh}, cf-*.txt in ~/.cache/fast-write-agent): with a formatting-free Report (JSON writer + check_detail + checked all without core::fmt) async/locale/query go to 0 bytes of core::fmt and atomics/cleanup/files/net/notes/tls/ui/ui-list were ALL harness even on main. Exe: net 12419->9240, tls 15098->9874, query 19532->12384. Decomposition on fast: JSON writer alone -200..-650; + check_detail(Arguments) up to -6835 (query); + checked's {e:?} -2600..-3300 for the rest.
- DX: clippy's format lints (write_literal, useless_borrows_in_formatting) and rustc's named_arguments_used_positionally fire on core::write! but not on the fast macro.
- Gates green: host test+clippy, SDK clippy --release, symbian-macros 23 unit tests.
- Emulator (pre-rebase, 7231c84 base): async 15, atomics 23, files 26, locale 8, notes 3, query 4, time 29, tls 45, cleanup 2, fmt 14, net 22 (peers up), ui-list 6 passed; shim writes no result by design. Notes files written through the fast writeln! read correctly (locale/time/async). fmt ticks run 2: Buf16 core 113 / fast 75, String 46 / 42.
- Coordinator: main moved to aae58bb (symbian-macros gains `strings` + deps, locale rewritten, experiment 100 taken). Rebase, use next free experiment number, re-measure against new main.
- Rebased onto aae58bb (13 commits; Cargo.lock conflicts regenerated with `cargo metadata`, lib.rs keeps `{main, strings}` + fast write export, locale keeps main's imports + `use symbian_std::writeln`). This slice is experiment 100.

## Decisions

- Shape: `macro_rules! write` in symbian_std (matches `$dst:expr, $fmt:literal, $($arg:expr),*`; anything else -> `::core::write!` verbatim so rustc's own errors stay) calling a proc macro with `$crate`. Receiver evaluated once by one method call on `$dst` (two-phase borrow like write_fmt); per-piece dispatch by autoref specialisation on a probe returning a tag; slow piece = a closure `|d, a| d.write_fmt(format_args!("{spec}", a))` written at the call site.
- Export at `symbian_std::{write, writeln}` (crate root), not in the prelude; a program opts in with one explicit `use symbian_std::{write, writeln};`. Report the conflict to the user.

## Dead ends

- Glob prelude export of `write`/`writeln` (E0659, see Findings).

## Next step

- Emulator: every report example + net peers + ui by hand; final measure (with fmt example); DX diagnostics check; docs (backlog 99, size-levers L8); delete wip.
