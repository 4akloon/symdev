# 96 — sizes, before and after

All figures are `.exe` bytes from `symdev build`, S60 3rd FP2 / `arm-symbian-e32`,
2026-09-21.

## What the localisation system itself costs

A `hello`-shaped program (`Buf16`, `write!`, `User::InfoPrint`, `User::After`), built
four ways in a throwaway `examples/sizeprobe`. The second column is the implementation
that was tried first and thrown away: `Language::current()` reading `User::Language()`
once into a `static AtomicU32` and loading it afterwards.

| the greeting comes from | shipped (no cache) | rejected (`AtomicU32` cache) |
|---|---|---|
| a plain `const &str`, no `locale!` at all | **3 187** | 3 187 |
| `locale!`, 1 language | **3 183** | 3 183 |
| `locale!`, 2 languages | **3 230** | 4 024 |
| `locale!`, 3 languages | **3 309** | 4 067 |

- **One language costs nothing**, and in fact four bytes less than writing the string as
  a `const`: the generated `get()` opens with a constant `if TRANSLATED.is_empty()`, LLVM
  folds it, and `User::Language()` is never imported.
- **A second language costs 47 bytes**, of which 19 are the French string itself.
- **The cache would have cost 794 bytes** — `AtomicU32::load(Relaxed)` compiles to
  `bl __atomic_load_4` on ARMv5TE (disassembled: `82d0: bl 9678 <__atomic_load_4@plt>`),
  and linking that pulls in `symbian-libcalls`' whole atomics object: thirty
  `__atomic_*` entry points, `AtomicLock`, `RFastLock::{CreateLocal,Wait,Signal}`,
  `__sync_synchronize`, 16 bytes of `.bss`, four more PLT entries, ~43 more dynamic
  symbols, `.text` +1 952.

## Every example, before and after this branch

Baseline: the same commit this branch left (`91d57c4`) in a second worktree whose path
is the same length, both built with each tree's own `symdev`.

| example | before | after | delta |
|---|---|---|---|
| alloc | 4 474 | 4 474 | 0 |
| async | 21 659 | 21 659 | 0 |
| atomics | 11 719 | 11 719 | 0 |
| files | 10 552 | 10 552 | 0 |
| hello | 3 187 | 3 187 | 0 |
| hello-raw | 752 | 752 | 0 |
| **locale** | — | **9 684** | new |
| net | 13 379 | 13 379 | 0 |
| notes | 15 298 | 15 298 | 0 |
| query | 20 120 | 20 120 | 0 |
| shim | 4 474 | 4 474 | 0 |
| spawnee | 3 208 | 3 208 | 0 |
| time | 20 583 | 20 583 | 0 |
| tls | 16 272 | 16 272 | 0 |
| ui | 12 844 | 12 844 | 0 |
| ui-list | 14 489 | 14 489 | 0 |
| std-hello | 73 637 | 73 631 | −6 (not this branch, below) |
| std-net | 53 498 | 53 358 | −140 (not this branch, below) |

**Every `no_std` example is unchanged to the byte.**

**The two `std` examples differ between the worktrees, and it is not this branch.**
Measured: with `pub mod locale;` commented out of *both* `symbian-core` and
`symbian-std` **and** `symbian-sys/src/euser.rs` reverted to the baseline's copy — that
is, with every line this branch adds switched off — `std-net` in this worktree still
builds to **53 358**, while the identical source in the baseline worktree builds to
**53 498**. Each figure is reproducible inside its own tree (two clean rebuilds apiece)
and the two trees' regenerated `Cargo.lock`s are identical. So the difference is a
property of the tree, not of the change; what property was not determined. Experiment 91
recorded the same instability for `std-hello` (73 617 against 73 633) and reached the
same conclusion.

`localedemo.exe` at 9 684 bytes is not comparable with `hello`: it carries `alloc`,
`String`, `core::fmt`, the file API and the `test_report` harness so that the
measurements can be read back off the drive. The cost of the localisation itself is the
first table.
