# WIP: rust-lld spike (experiment 109)

Question: can a Rust SDK app be built with no GCCE on the dev machine — prebuilt shims +
GCC runtime archives shipped, final link by `rust-lld -flavor gnu` instead of GNU ld?

Scratch (outside git): `~/src/rust-lld-spike/` (work/ = projects, logs, ELFs).
Branch: `rust-lld-spike` (worktree `~/worktrees/symdev/rust-lld-spike`). No product code changes.

## Status
- [ ] 0. Read context (shim header, driver link code, experiments)
- [ ] 1. Capture GNU ld argv for console / async / GUI Rust apps
- [ ] 2. Archive members pulled in (--trace / -Map), sizes, licences
- [ ] 3. Replay with rust-lld; adaptations; readelf comparison
- [ ] 4. elf2e32 on lld ELF; E32 compare
- [ ] 5. EKA2L1 run + screenshot; leave probe
- [ ] 6. Effort estimate; experiment 109 record

## Facts

## Dead ends

## Next step
Read context files.
