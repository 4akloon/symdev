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

## Decisions
- `check_detail(name, ok, detail: impl FnOnce(&mut String))`; call sites write `detail!(...)`
  where they wrote `format_args!(...)` — `symbian_std::detail!` expands to a closure that
  runs the fast `write!` (with `core::fmt::Write` imported inside, which the fast
  macro's never-called slow closure needs to type-check).
- `checked<T, E: Evidence>`: trait `test_report::Evidence` (`show(&self, &mut String)`,
  `shown() -> String`) writes Debug's text without core::fmt for SymbianError, io::Error,
  SystemTimeError, AccessError, (), ints, bool, str/String, Option, Result, slices, Either,
  and `Hex(usize)` (= `{:#x}`). io::Error records "KErrNotFound (-1)" — name and TInt;
  the std-kind prefix of its Debug ("NotFound (...)") is dropped (would need a name table).
- JSON/path hex via a private `push_hex`; test_report.rs split into test_report/{mod,json,evidence}.rs.

## Dead ends

## Next step
- Read experiment 101, size-levers core::fmt, test_report.rs, symbian-fmt; measure baseline.
