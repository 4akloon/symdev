# T3 rcomp barrel-order fix

Date: 2026-09-18
Branch: `t3-rcomp`
Worktree: `/home/genius/worktrees/symdev/t3-rcomp`
Fix commit: `6ab0a5b1aa9bc35da1c5872ae3eb1b19a08cb435`

## Finding

Crate-root `mod rcomp;` in `crates/symdev-build/src/lib.rs` had been appended after `mod uidcrc;`. Module barrels are alphabetical; `pub use` stays append-only.

## Change

Moved `mod rcomp;` between `mod pkg;` and `mod sis;`. Did not reorder `pub use`.

Confirmed crate-root `mod` order:

```
mod bld;
mod driver;
mod mmp;
mod model;
mod pkg;
mod rcomp;
mod sis;
mod toolchain;
mod uidcrc;
```

`rcomp/mod.rs` has no sibling production mods (`mod tests` only). No change there.

`pub use rcomp::{RcompTool, RscUid};` remains last (append-only).

## Tests

`cargo test --workspace --offline` in this worktree: **pass** (120 + 3 + 19 + 5 + 17 unit/integration tests).

## Merge

Did not merge to `main`. Pushed `origin/t3-rcomp` only. Worktree kept.
