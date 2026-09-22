# harness-fmt (WIP)

Task: make `symbian_std::test_report::Report` stop linking `core::fmt` (detail line,
`{e:?}` in `checked`, `{:08x}` in JSON) with a one-line call API and unchanged JSON schema;
re-enable fast `write!`/`writeln!` in async, locale, query, time only where it shrinks;
measure every non-std example before/after; every report-writing example passes
`symdev test --emulator`; prove a deliberate failure still fails. Record the experiment
in the backlog, update size-levers.md.

## Findings
- Baseline (branch point 5eb3beb), `.exe`/.text/.rodata/core::fmt bytes: alloc 3773/4936/112/2020,
  async 20451/32176/2732/5844, atomics 10723/14684/2408/2908, cleanup 5783/7884/307/2480,
  files 11102/14688/2608/2908, fmt 100667/237988/4683/10700, hello-raw 808/292/44/0,
  hello 1245/884/28/0, locale 12045/15192/2436/2936, net 12434/16660/2288/2908,
  notes 14670/18952/1904/3652, panic 2010/1996/36/0, query 19392/22368/5892/4748,
  shim 4517/5712/128/0, spawnee 3079/3756/568/0, time 13561/17384/3004/4840,
  tls 15098/20688/4196/3664, ui-list 13907/16684/2104/2484, ui 12975/16492/1704/2484.
  Script: ~/.cache/harness-fmt-agent/measure.sh (res-base.txt).
- Error types given to `checked` in examples: io::Error, SymbianError, SystemTimeError, `()`.
  Today's Debug: io::Error "NotFound (KErrNotFound (-1))", SymbianError "KErrNotFound (-1)".
- Host JSON reader: crates/symdev-emulator/src/results.rs (+ results/tests.rs, json/tests.rs).
- Step 1 (new harness API, every `format_args!` → `detail!`, no other example change):
  core::fmt gone from atomics, cleanup, files, net, ui, ui-list (−1 217…−1 725 each);
  still in async/notes/query/tls (their Debug details), time/locale (own write!), alloc, fmt.
  async +1 097, notes +196, query +330, tls +606 (Debug still linked + new code).
  In atomics `json::escape_into` is 680 B and `push_hex` 232 B — String::push per char.
- JSON escape byte-run rewrite: escape_into 680→632 B (a 5-arm match, not worth more).
- Step 3 (Debug details → `Evidence::shown()`, `{:#x}` → `Hex(..).shown()`): core::fmt 0 in
  atomics cleanup files net notes tls ui ui-list; left in async 1772, query 2352, locale 2152,
  time 3816 (their own `core::write!`), alloc 2020, fmt 10604. Every example shrinks:
  query −3870, notes −1866, atomics −1794, net −1787, files −1726, tls −992, async −732.
- Fast write! opt-in, same harness, core vs fast: async 19719→18603 (−1116), locale
  11584→10233 (−1351), query 15522→14028 (−1494), time 11591→10256 (−1335) — time only
  after its own three `{:?}` of `Result<_, i32>` in the notes became `.shown()` (same text).
  All four then link no core::fmt.
- alloc keeps core::fmt (2020 B): its own `{:x}` (LowerHex) into a Buf16 — not the harness
  (alloc writes no report).
- Final sizes = step 3 + opt-in (res-final.txt); alloc −6 without touching it (no report).
- Emulator, `symdev test --emulator`, all pass: async 15, atomics 23, cleanup 2, files 26,
  fmt 14, locale 7, net 22 (peers 18974/18975), notes 3, query 4, time 29, tls 45, ui 3,
  ui-list 6, std-hello 50, std-net 31 (peers 18984/18985). Details read as Debug did:
  "Ok(Ok(()))", "Err(KErrNotReady (-18))", "Ok(Left(Ok(())))", "[100, 400], …", tls "0x10000004".
- Deliberate failure (cleanup, not committed): `checked` of fs::metadata on a missing path +
  `check_detail(false, detail!("{} of {} \"quoted\"\t", 1, 2))` → "2 failed, 2 passed",
  exit 1, detail "KErrNotFound (-1)", JSON escaped `\"` and `\t` correctly.
- std-hello/std-net Cargo.lock were stale since experiment 101 (no symbian-fmt); the build
  refreshed them.

- `check_detail(name, ok, detail: impl FnOnce(&mut String))`; call sites write `detail!(...)`
  where they wrote `format_args!(...)` — `symbian_std::detail!` expands to a closure that
  runs the fast `write!` (with `core::fmt::Write` imported inside, which the fast
  macro's never-called slow closure needs to type-check).
- `checked<T, E: Evidence>`: trait `test_report::Evidence` (`show(&self, &mut String)`,
  `shown() -> String`) writes Debug's text without core::fmt for SymbianError, io::Error,
  SystemTimeError, AccessError, (), ints, bool, str/String, Option, Result, slices, Either,
  and `Hex(u32)` (= `{:#x}`). io::Error records "KErrNotFound (-1)" — name and TInt;
  the std-kind prefix of its Debug ("NotFound (...)") is dropped (would need a name table).
- JSON/path hex via a private `push_hex`; test_report.rs split into test_report/{mod,json,evidence}.rs.

## Dead ends

## Next step
- Full re-measure; emulator runs of every report-writing example; deliberate failure proof;
  gates; backlog experiment + size-levers; delete this file.
