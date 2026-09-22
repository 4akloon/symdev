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

## Decisions

- Shape: `macro_rules! write` in symbian_std (matches `$dst:expr, $fmt:literal, $($arg:expr),*`; anything else -> `::core::write!` verbatim so rustc's own errors stay) calling a proc macro with `$crate`. Receiver evaluated once by one method call on `$dst` (two-phase borrow like write_fmt); per-piece dispatch by autoref specialisation on a probe returning a tag; slow piece = a closure `|d, a| d.write_fmt(format_args!("{spec}", a))` written at the call site.

## Dead ends

## Next step

- Identity tests (recording sink, call sequences), then Buf16 Sink impl in symbian-core, prelude export, measure.
