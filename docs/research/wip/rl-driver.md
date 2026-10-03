# RL3 — wire rust-lld into symdev's Rust build (branch `rl-driver`)

Task from the lead (2026-10-03): default Rust linker = rust-lld, `SYMDEV_RUST_LINKER=gnu`
keeps GNU ld byte-identical; prebuilt rust-sdk => no GCCE; SDK lld-fix cache; ImportStubs
second link; `--defsym=symrs_uid3`. Record as experiment 113. Scratch: `~/src/rl-driver-scratch/`.

## Status
- [ ] read specs/code
- [ ] plan

## Findings

## Decisions

## Dead ends

## Next step
Read exp 109/112, wip/rust-shims.md, v0.2.md, spec §3/§12, driver code.
