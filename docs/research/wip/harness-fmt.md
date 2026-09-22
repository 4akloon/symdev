# harness-fmt (WIP)

Task: make `symbian_std::test_report::Report` stop linking `core::fmt` (detail line,
`{e:?}` in `checked`, `{:08x}` in JSON) with a one-line call API and unchanged JSON schema;
re-enable fast `write!`/`writeln!` in async, locale, query, time only where it shrinks;
measure every non-std example before/after; every report-writing example passes
`symdev test --emulator`; prove a deliberate failure still fails. Record the experiment
in the backlog, update size-levers.md.

## Findings

## Decisions

## Dead ends

## Next step
- Read experiment 101, size-levers core::fmt, test_report.rs, symbian-fmt; measure baseline.
