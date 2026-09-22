# named-panic (WIP)

Task: replace `User::Exit(-1)` in the `no_std` `#[panic_handler]` (symbian-runtime) with
`User::Panic(category, reason)` — user's choice «User::Panic з категорією» (parity with C++
on diagnosability). Bind `User::Panic` in symbian-sys, choose category (<= KMaxExitCategoryName)
and reason, decide OOM (exit vs panic), measure cost per no_std example vs main, find reclaimable
levers, observe the panic in the EKA2L1 log (Kernel:trace, restore log-filter), compare with a
C++ `User::Panic` in a scratch copy of docs/research/cpp-parity/hello. Experiment entry in the
backlog (next free number on main: 99). Branch `named-panic`, base main 7231c84. Do not merge.

## Findings

## Decisions

## Dead ends

## Next step

Read the current handler, symbian-sys euser bindings, e32std.h for User::Panic and KMaxExitCategoryName.
