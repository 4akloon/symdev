# cargo build / run / test — spike + plan (branch `cargo-run`)

Task (lead, 2026-10-03): phase 1 = spike of spec §11
(`docs/superpowers/specs/2026-10-03-cargo-build-run-design.md`), recorded as experiment 114 §1
in `docs/research/experiment-backlog.md`; phase 2 = plan via superpowers:writing-plans at
`docs/superpowers/plans/2026-10-03-cargo-build-run.md`. Scratch: `~/src/cargo-run-scratch/`.
No push to main / merge / tag / publish; pushing `cargo-run` is allowed.

## Status
- [ ] Q1 linker argv + env (dev, release, harness=false test)
- [ ] Q2 same bytes from rustc's link inputs (hello, ui, async)
- [ ] Q3 libcalls as an ordinary dependency
- [ ] Q4 patched std from config alone (rust-std)
- [ ] Q5 dev profile
- [ ] Q6 test binary vs main binary signal
- [ ] exp 114 §1 written
- [ ] plan written + committed

## Facts

## Dead ends

## Next
Read exp 109–113 format, `crates/symdev-build` Rust build path, set up env.
