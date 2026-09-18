# T2 native unsigned package — replace Wine makesis

Date: 2026-09-18
Branch: `t2-package-native`
Worktree: `/home/genius/worktrees/symdev/t2-package-native` (kept)
Primary checkout: `/home/genius/projects/symdev` still `main`
Did not edit `unsigned.rs` / `controller.rs` (owned by worktree `t2-sisx`).

## Result

`symdev package` writes unsigned `.sis` with `encode_unsigned_sis` → `SisUnsigned::bytes()` from the project's exe + pkg metadata (name, UID3, version, vendor, dest `!:\sys\bin\<name>.exe`, live SHA-1, file size, capability bits). Wine **makesis is not spawned**. Wine **signsis** and **makekeys** stay.

Hello.pkg fixture (experiment 7) plus hello.exe bytes byte-equals frozen Wine `hello.sis` (4000 bytes, SHA-256 `06f39722f5f911d59c119d126c223eabd7b3ec4c81b3175bbebc3f3eb7855232`). A different name/UID does not emit that golden. Unknown capabilities (e.g. `AllFiles`) error: `SIS capability bits not yet derived from pkg: …`.

No E52 claim. Never `-fPIC`. No SDK / `.sis` / `.cer` / `.key` committed. Crate-root `lib.rs` exports are append-only (`SisUnsignedSpec`, `encode_unsigned_sis` after `SisSignatures39`). Experiment 37 left for the SISX worktree.

## SHAs

| What | SHA |
|------|-----|
| origin/main base | `99d942718ee79514af3338da8a9de90b63f3663d` |
| Native encode + experiment 38 | `8940cb5cf9e6c2711123640543f8f9a68c018650` |

Not merged to main. Feature branch only.

## Still Wine

| Tool | Status |
|------|--------|
| `makesis` | **not called** from `SisPackage::package` (argv helper + `SisTools.makesis` path kept) |
| `makekeys` | Wine, recorded argv, unless `signing.cert` + `signing.key` exist |
| `signsis` | Wine, recorded argv → `.sisx` |
| `SisTools::from_env` | still requires `SYMDEV_EPOCROOT` (signsis/makekeys PE paths) |

Live `package()` stamp is UTC now (Wine makesis datetime is not in pkg). Hello golden tests inject the recorded stamp `2026-08-17 15:18:24`. TYPE=SA one-file words `0x21` / `0x14` / `0x0d` / `0x1a` stay recorded for the current `render_pkg` grammar.

## Tests

TDD: tests failed with missing `encode_unsigned_sis` / `SisUnsignedSpec`, then passed. Default `cargo test` does not spawn Wine.

`cargo test --workspace --offline`:

- `symdev-build` 102 passed (5 new: hello fixture equality, other-app UID not hello golden, unknown cap error, live SHA-1 vs experiment 29, native `.sis` written before Wine signsis)
- `symdev` unittests 3 passed
- `cli` 19 passed (existing missing-uid3 / missing-e32 / missing-EPOCROOT / short-password paths unchanged)
- `symdev-core` 5 passed
- `symdev-manifest` 17 passed

## Docs

- Experiment 38 (append-only): `docs/research/experiment-backlog.md`
- This report: `.superpowers/sdd/t2-package-native-report.md`
