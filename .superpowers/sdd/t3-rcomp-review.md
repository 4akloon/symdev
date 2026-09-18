# T3 rcomp review — `origin/t3-rcomp` vs `origin/main`

Date: 2026-09-18
Reviewer: code-review subagent
Branch: `t3-rcomp` @ `66f02514f25f17253b6b4f86c9ced30c23a65988` (worktree `/home/genius/worktrees/symdev/t3-rcomp`)
Base: `origin/main` `d04e7c5643e3aae51ac623c4406dcb7b7987a3be`
Commits: `b7c0f76` Match Wine rcomp RSC UID headers with RscUid; `66f0251` Record the T3 rcomp native UID-header report.
Did not merge, push, commit, or edit implementation.

**Verdict: Important findings (fixed on branch)**

Not Blocking. Native 16-byte UID headers match Wine rcomp goldens; hex testdata is frozen and byte-equal to experiment 9/41 binaries; domain type does not know Wine; crate-root `pub use` is append-only. The Important layout miss (`mod rcomp` after `uidcrc`) is fixed in `6ab0a5b1aa9bc35da1c5872ae3eb1b19a08cb435` — crate-root `mod` is now alphabetical (`pkg` → `rcomp` → `sis`). See `.superpowers/sdd/t3-rcomp-barrel-fix.md`.

## Findings

### Important

1. **`mod rcomp` is not alphabetical in the crate-root barrel.** **Fixed in `6ab0a5b`.** `mod rcomp;` now sits after `mod pkg;` and before `mod sis;`. Crate-root `pub use` remains append-only (`pub use rcomp::{RcompTool, RscUid};` still last).

### Minor

2. **Golden body after byte 16 is pinned but not hashed in tests.** Driveinfo 74 and filebrowse 109 lengths are asserted, and the first 16 bytes are compared to `RscUid::bytes()`. SHA-256 of the full hex decode is documented in experiment 41 / the implementer report, and this review decoded both `.rsc.hex` files to the claimed hashes and to the on-disk experiment-9 (`$HOME/src/symdev-experiment-5/`) and experiment-41 `.rsc` files. A later edit that keeps length and header but mutates the RSS body would still pass. Optional: assert SHA-256 in the tests. Not required for this UID-header slice.

### Not findings (checked)

- No public free functions on the production API (`RscUid` / `RcompTool` methods only).
- `RscUid` has no Wine paths; argv lives on `RcompTool` (same split as `UidCrc` / `UidCrcTool`).
- No Wine/`Command` spawn in default tests; recorded paths are `/usr/bin/wine` + `/sdk/epoc32/tools/rcomp.exe` string compares only.
- Argv is the experiment-9 / experiment-41 recorded tokens (`-u`, glued `-o`/`-s`/`-i`, glued `-h` after `-o`). `-force` / `-{uid2,uid3}` were not invented.
- Hex goldens in `crates/symdev-build/src/rcomp/testdata/*.rsc.hex`; no `.rsc` / `.rss` / `.rpp` / `.rsg` / `.sis` / `.sisx` / `.cer` / `.key` in git.
- Driveinfo 74 bytes, SHA-256 `10bd8e607b9f166629ac1e9285d6abbad24e88972a885a15062f8680d575b7d8`. Filebrowse 109 bytes, SHA-256 `437dbc17eef7b26d9650917b408d22922f96e7ed888e2916035f56eb010f0b60`. Headers `6b4a1f10…` match `UidCrc` of UID1 `0x101f4a6b` + the recorded UID2/UID3/checked words.
- Experiment 41 appended after 39; experiment 40 left unused (reserved for makekeys in the 41 write-up). Prior backlog sections 1–39 untouched.
- Full RSS body encode, non-empty `.rsg`, and `START RESOURCE` / CLI wiring stay out of scope. `-h` `.rsg` on disk is 0 bytes; not committed.
- `cargo test -p symdev-build --offline rcomp`: 6 passed. Host `rustc 1.98.1`, workspace `edition = "2024"`. No new crate deps.

## OOP / free-function check

**Pass.** Public surface is types + methods:

- `RscUid { uid2, uid3 }` — `new`, `UID1`, `crc() -> UidCrc`, `bytes()` (delegates to `UidCrc`, no Wine).
- `RcompTool { wine, rcomp }` — `new`, `args`, `args_with_header`.

Mirrors `SisUid` / `UidCrc` + `UidCrcTool`. No `pub fn` encode helpers. Test-only `parse_hex` / `driveinfo_rsc` / `filebrowse_rsc` / `recorded_tool` match existing `#[cfg(test)]` helpers elsewhere in `symdev-build` (including duplicated `parse_hex`); they are not the public API the “bare functions” complaint targets.

## pub use / module layout check

| Site | Rule | Result |
|---|---|---|
| Crate-root `pub use` | Append-only | **Pass.** Only addition is `pub use rcomp::{RcompTool, RscUid};` after `uidcrc`. Brace order `{RcompTool, RscUid}` is alphabetical. Existing `sis { … }` list untouched. |
| Crate-root `mod` | Alphabetical barrel | **Fixed (`6ab0a5b`).** `mod rcomp` sits between `pkg` and `sis`. |
| `rcomp/mod.rs` | New module, no inner barrel | **Pass.** Single file, two types. |

## Merge-ready after coordinator sequential merge?

**Yes after `6ab0a5b`.** The Important `mod rcomp` placement is fixed on this branch (`6ab0a5b1aa9bc35da1c5872ae3eb1b19a08cb435`: `pkg` → `rcomp` → `sis`; `pub use` still append-only). Ready for the coordinator to land sequentially (do not merge this review). Notes for that merge:

- `lib.rs` `pub use` can stay a tail append; only `mod rcomp` needs an alphabetical insert (`pkg` → `rcomp` → `sis`).
- `docs/research/experiment-backlog.md` appends `## 41` after `## 39`. If the makekeys branch writes `## 40`, keep 40 between 39 and 41; do not let 41 occupy the 40 slot.
- Independent of makekeys implementation. No `main` merge performed here.

## Ponytail (over-engineering only)

Lean already. Ship the types. `args` / `args_with_header` duplicate a six-token prefix (~12 lines); not worth a layer until a third recorded rcomp argv appears. `net: 0` required cuts.
