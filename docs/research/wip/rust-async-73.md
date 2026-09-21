# WIP: step 73 — Rust async/await on Symbian Active Objects

Task: a single-threaded executor on `CActiveScheduler` (own or join), `TRequestStatus` -> `Waker`, `RTimer` sleep, and an example where two 300 ms timers awaited concurrently finish in ~300 ms.

## Findings

## Decisions

## Dead ends

## Next step

- Read the governing docs (design spec §6a/§7/§8/§11, avkon-rust-spec active scheduler, eka2-concurrency, experiments 78/80/84) and `symbian-core/src/net/`.
