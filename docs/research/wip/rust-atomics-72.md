# WIP: step 72 — real atomics, Arc, Mutex, Once on ARMv5TE (no LDREX/STREX)

Task: ship `shims/common/symrs_atomic.cpp` over an `RFastLock`, raise the target to
`max-atomic-width: 32` + `atomic-cas: true` in the same commit, add `symbian_std::sync`
(`Mutex`, `MutexGuard`, `Once`, re-export `Arc`) and `symbian_std::thread`
(`spawn`/`join` over `RThread`), and an `examples/atomics` that proves the counts through
`symdev test --emulator`. Settle the post-thread-exit access violation at 0x8000A4.

## Findings
- Baseline E32 sizes reproduced on this worktree before any change: hello 3187, hello-raw 752, alloc 4320, shim 4475, files 10423 — exactly the figures in the brief.

## Decisions

## Dead ends

## Next step
- Read the survey and the spec sections, then reproduce the undefined-symbol link.
