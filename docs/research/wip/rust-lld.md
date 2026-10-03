# rust-lld productisation (branch `rl-elf2e32`)

Task: (A) elf2e32 accepts lld ELFs (5 rules of experiment 109 §3, gated on lld detection,
labelled; GNU goldens unchanged; lld ELF hex golden). (B) 8-byte import stubs with lld
(`ImportStubs` type), proven on hello/async/shim/ui vs GNU sizes + EKA2L1; experiment 112.

Worktree `/home/genius/worktrees/symdev/rl-elf2e32`; scratch `~/src/rl-scratch/`.
Spike: `~/src/rust-lld-spike/` (fork `elf2e32-fork/`, grep `SPIKE 109`).
Do NOT edit: `crates/symdev-build/src/driver/{rust_build,libcalls}.rs`, `crates/symdev-sdk`,
`symbian-rs/shims`, packages repo.

## State
- [ ] A: read spike fork + product elf2e32
- [ ] A: lld detection rule
- [ ] A: five rules + tests
- [ ] A: lld golden
- [ ] B: lld options investigation
- [ ] B: stub mechanism + ImportStubs type
- [ ] B: four apps sizes / elf2e32 / EKA2L1
- [ ] experiment 112 entry

## Findings

## Dead ends

## Next step
