# rsc-reader (wip)

Task: replace the `RResourceFile` path in `symbian-core::locale` (experiment 99) with a
no-leave reader over `RFile` that understands exactly the compiled `.rsc` subset our own
`symdev-rcomp` emits for `<app>_strings.rsc` / `.r<code>` (BUF8 only, uncompressed).
Keep `Str::at`, `Str::get() -> Result<Text>`, `Text: Deref<Target = str>`. File choice stays
`BaflUtils::NearestLanguageFile` (verify non-leaving). Unknown layout = error naming it.
Delete `symrs_rsc.cpp` if unused. Measure code (all non-std examples, locale vs C++ 6 647),
exception runtime by symbol, heap, get tick cost. Emulator at language 1 and 2 (restore 1).
Record an experiment in the backlog.

## Findings

## Decisions

## Dead ends

## Next step

- Read experiment 99, rcomp writer, current locale module.
