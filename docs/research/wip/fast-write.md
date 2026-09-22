# fast-write

Task: a `write!`-compatible macro in `symbian_std::prelude` that turns plain `{}` of strings/integers into direct appends and falls back to `core::write!` otherwise, byte-identical output; measure against C++.

## Findings

- Baseline sizes on main 7231c84 (exe/text/rodata) in ~/.cache/fast-write-agent/res-base.keep; hello 2567. Measure script: ~/.cache/fast-write-agent/measure.sh <label>.
- Most examples reach core::fmt through `format_args!` into `Report::check_detail`, not `write!`. Direct `write!` users: hello (Buf16), alloc, locale/async/time (String `notes`), query (String), test_report itself (String).
- core's integer Display writes the sign with write_char then digits with write_str, so on an overflowing sink `-` can land alone: a fast path must reproduce that partial write.
- Buf16::append_num's `decimal_len` divides u64 by 10 at run time -> possible `__aeabi_uldivmod`; check.

## Decisions

- Shape: `macro_rules! write` in symbian_std (matches `$dst:expr, $fmt:literal, $($arg:expr),*`; anything else -> `::core::write!` verbatim so rustc's own errors stay) calling a proc macro with `$crate`. Receiver evaluated once by one method call on `$dst` (two-phase borrow like write_fmt); per-piece dispatch by autoref specialisation on a probe returning a tag; slow piece = a closure `|d, a| d.write_fmt(format_args!("{spec}", a))` written at the call site.

## Dead ends

## Next step

- Read size-levers.md, symbian-macros, prelude; design mechanism.
