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

## Member ABI — OBSERVED (2026-09-20)

Probe `scratchpad/shim70/memberabi{,2}.cpp`, compiled with the observed GCCE argv, `objdump -d --reloc`:

- `d->Append(*s)` from `probe_append(TDes16*, const TDesC16*)`: `bl _ZN6TDes166AppendERK7TDesC16` with **no register shuffle** — `this` is r0, args follow.
- `d->AppendNum((TInt64)n)` from `probe_appendnum(TDes16*, TInt)`: `movs r2,r1; asrs r3,r1,#31; bl _ZN6TDes169AppendNumEx` — the 64-bit arg lands in r2:r3, skipping r1: plain AAPCS with `this` prepended as argument 0.
- `d->Num(v)` from `probe_num(TDes16*, TInt64)`: no shuffle at all (v already in r2:r3).
- `d->Find(*s)` from `probe_find(const TDesC16*, const TDesC16*)`: no shuffle, result in r0.
- **Exception, also observed:** `d->Left(n)` returning `TPtrC16` by value is **sret** — `mov r0,sp; movs r1,r0(this); movs r2,n`. The hidden return slot is argument 0 and `this` moves to argument 1.
- `MaxLength()`/`Length()` are inline (`ldr r0,[r0,#4]` / `[r0,#0]` masked) — no call at all.

So: a non-virtual, non-static member with scalar/pointer arguments and a scalar or void return is callable from Rust as `extern "C" fn(this, ...)`. No shim needed for that class.
