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

- Layout (writer `crates/symdev-rcomp/src/rsc.rs` `RscCompiled::rsc_bytes`): 16 B UidCrc (UID1 0x101f4a6b,
  UID2, UID3, CRC), 1 flag byte (0x01 = `FLAG_UID3_FROM_NAME`, set when no UID3 statement — our
  strings source has `NAME STRS` and no UID3), u16 LE largest uncompressed resource, ceil(n/8)
  bytes one bit per packed resource, the resource bodies, then n+1 u16 LE offsets: entry i = start
  of resource i+1, entry n = start of the index itself (= end of last body). So the last 2 bytes
  of the file locate the index; n = (size - index_start)/2 - 1. Resource k (1-based) = [idx[k-1], idx[k]).
- Strings source (`crates/symdev-build/src/strings_resources.rs`): index 1 = SYMDEV_SIG (LONG 4 +
  SRLINK self, 8 B); index 2+i = key i in byte order, `BUF8` = raw UTF-8, never packed
  (rcomp-spec §3.1, §1.4: packed bit only if a compressible 16-bit text remains).
- `BaflUtils::NearestLanguageFile(const RFs&, TFileName&)` is non-leaving, verified in the ROM the
  emulator runs (rm-469 `z:\sys\bin\bafl.dll`, ROM image header 0x78 B, export 276 per
  bafl.dso -> 0x803fb20d, Thumb; it tail-calls the 3-argument overload, export 453). Transitive
  call closure (34 local functions, the finder's two vtables 0x80401ed0/0x80401e3c resolved by
  hand, every indirect call is through them) reaches only: TDes16/TPtrC16/TBufBase16 members,
  TParsePtrC/TParseBase::NameAndExt, RFs::Entry, RDir::Open/Read/Close, TEntry ctor,
  User::Language, TLocale ctor, UserSvr::DllTls, HAL::Get, __aeabi_idivmod, __ARM_switch8,
  operator delete (scppnwdl). No User::Leave*, no `L` function. Scripts in the session scratchpad
  (closure.py), not committed.

- Pure reader `symbian-core/src/locale/layout.rs` (core only), host-tested unchanged via `#[path]`
  from `crates/symdev-build/tests/strings_layout.rs` against real rcomp output: 70 keys (9-byte
  bitmap), empty value, non-ASCII, signature; refusals are the writer's own output where possible
  (UID3 statement -> flags 0; a BUF text -> packed bit). 9 tests pass.
- Baseline sizes (this worktree at eb39761): ~/.cache/rsc-reader-agent/res-base.txt.

- Reader wired (commit after eb39761+2): locale exe 12 045 -> 9 921 B; nm shows no __gxx_personality_v0,
  _Unwind_*, typeinfo XLeaveException, symrs_rsc_* (base had them, 3 736 B by those symbols).
- Emulator lang 1: 7 passed, English; lang 2: 7 passed, French. Heap: open file +0 cells/+0 B
  (7/364 -> 7/364), held greeting +1/+36 B, freed back. Patched build .r02 "Bonjour"->"BONJOUR",
  repackaged: lang 2 printed "BONJOUR depuis Rust" (test failed as it must) -> French is read from .r02.
  config restored to language: 1.

## Decisions
- Hold only RFile handle + {index_at, count} (no heap); per get: 2 positional reads. (Pending
  measurement of the index-in-heap alternative + tick cost.)

## Dead ends

## Next step

- Measure all examples; tick cost of get (old vs new, and index-in-heap variant); host gates; backlog entry.
