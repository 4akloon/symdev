# WIP: step 70 — the C++ shim that lets Rust reach leaving Symbian APIs

Task: build `symbian-rs/shims/`, compile it into every Rust app through `RustBuild`, and prove a trapped leave returns `Err` to Rust instead of killing the process.

## Findings

- Spec §7 already says: shims live in `shims/s60/`, `extern "C" TInt symrs_<name>(...)`, compiled by GcceBuild's argv, linked as a static archive; §11 step 70 passes when a leaving API returns `Err` and the process survives.
- Exp 76: TRAP inside the shim turns `User::Leave(-6)` into return value -6 and the process continues (A+). Without it the process dies silently (A-).
- Exp 77: the 107 KB mixed C++/Rust case is already fixed by naming `-l:euser.dso -l:drtaeabi.dso` before the Rust archive. Baseline sizes to keep: hello 3187, alloc 4324.
- Exp 69 open item: every `TPtrC16` ctor and `TDes16::Copy/Append/Format` are `IMPORT_C` *non-static members*; the C++ member ABI (`this` in r0) has not been observed here.
- Spec §7 names files as the first real API after alloc: `RFs::Connect`, `RFile::Replace/Open/Read/Write/Close` are non-leaving. So step 71's leaving calls must come from elsewhere.

## Decisions

## Dead ends

## Next step

- Read the spec and the backlog entries, then plan.
