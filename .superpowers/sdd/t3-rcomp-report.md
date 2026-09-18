# T3 rcomp — native RSC UID encode from SDK goldens

Date: 2026-09-18
Branch: `t3-rcomp`
Worktree: `/home/genius/worktrees/symdev/t3-rcomp` (kept)
Primary checkout: `/home/genius/projects/symdev` still `main` (untouched)
Base: `origin/main` `d04e7c5643e3aae51ac623c4406dcb7b7987a3be`

## Result

**`symdev` does not invoke `rcomp` today.** MMP `START RESOURCE` keeps inner lines only (the `.rss` filename is dropped). `GcceBuild`, `Toolchain`, and the CLI (`new` / `build` / `package` / `deploy`) have no rcomp path or clap verb. Hello templates have no `.rss`.

SDK `rcomp.exe` usage (Wine, no-arg, experiment 41; same string as experiment 9):

```
Usage: rcomp [-vpul] [-force] [-oRSCFile] [-{uid2,uid3}] [-hHeaderFile] [-sSourceFile] [-iBaseInputFileName]
```

`u` = Generate Unicode resource binary. First native slice is the 16-byte UID prefix of a Unicode `.rsc`, via existing `UidCrc` (not a full rcomp). Types + methods: `RscUid`, `RcompTool`. No Wine in default `cargo test`. No clap verb. Never `-fPIC`. No E52. rcomp C not copied.

## SHAs

| What | SHA |
|---|---|
| origin/main base | `d04e7c5643e3aae51ac623c4406dcb7b7987a3be` |
| Native `RscUid` + experiment 41 | `b7c0f764ee804a1e14311d6fa5fc5d86583a5c3f` |
| Frozen `driveinfo_reg.rsc` | SHA-256 `10bd8e607b9f166629ac1e9285d6abbad24e88972a885a15062f8680d575b7d8` (74 bytes) |
| Frozen `filebrowseapp_reg.rsc` | SHA-256 `437dbc17eef7b26d9650917b408d22922f96e7ed888e2916035f56eb010f0b60` (109 bytes) |
| `-h` `.rsg` (both) | 0 bytes; SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` (empty; **not committed**) |

Feature-branch tip SHA is the report commit on this branch.

## How rcomp would be invoked

It would not. Wave 0/1 still “shell out to legacy rcomp/epocrc” in research notes, but no argv is wired. Recorded Wine argv (experiment 9, re-run 41) if a later slice shells out:

```
WINEPATH=$EPOCROOT/epoc32/tools \
wine $EPOCROOT/epoc32/gcc/bin/cpp.exe -nostdinc -undef -C -D_UNICODE \
  -I 'Z:\…\epoc32\include' in.rss -o in.rpp
WINEPATH=$EPOCROOT/epoc32/tools \
wine $EPOCROOT/epoc32/tools/rcomp.exe -u -oout.rsc -sin.rpp -iin.rss
```

`WINEPATH` is required so rcomp can spawn sibling `uidcrc.exe`. Host `epocrc.pl` still does not run (FindBin `\` rewrite).

## Pinned goldens

Wine re-compile of copied SDK example `_reg.rss` (not authored; not committed) in `$HOME/src/symdev-experiment-41/` (outside git). `.rsc` byte-equal experiment 9.

| File | Header (16 bytes) | UID1 | UID2 | UID3 | checked |
|---|---|---|---|---|---|
| `driveinfo_reg.rsc` | `6b4a1f1021801f10f40100a0b40cc8f0` | `0x101f4a6b` | `0x101f8021` | `0xa00001f4` | `0xf0c80cb4` |
| `filebrowseapp_reg.rsc` | `6b4a1f1021801f10a60000e86964350a` | `0x101f4a6b` | `0x101f8021` | `0xe80000a6` | `0x0a356469` |

In-crate hex: `crates/symdev-build/src/rcomp/testdata/*.rsc.hex`. Native encode covers the header only; the rest of the `.rsc` is pinned, not produced. `-h` from usage emitted an empty `.rsg` (no `NAME` in registration RSS).

## Tests

TDD: `RscUid` / `RcompTool` missing (E0425/E0433), then 6 passed.

`cargo test --workspace --offline` (after GREEN):

- `symdev-build` 120 passed (6 new: driveinfo header + checked, filebrowse header, UID1 constant, experiment-9 argv, experiment-41 `-h` argv)
- `symdev` unittests 3 passed
- `cli` 19 passed
- `symdev-core` 5 passed
- `symdev-manifest` 17 passed

No Wine. No SDK / `.rss` / `.rpp` / `.rsc` binaries / `.cer` / `.key` committed.

## Docs

- Experiment 41 (append-only; 40 reserved for makekeys): `docs/research/experiment-backlog.md`
- This report: `.superpowers/sdd/t3-rcomp-report.md`

## Blockers / out of scope

- Full rcomp (RSS parse, STRUCT, resource body/index after byte 16)
- Non-empty `.rsg` (needs a `NAME`d RSS; hello-adjacent examples needed prior `.rsg`/`.rls` and were not invented)
- Wiring `START RESOURCE` into `build` / a clap verb
- `epocrc.pl` on Linux
- E52 / emulator (experiments 10/11 remain skip)
- Independent of makekeys (experiment 40)

No merge to `main`. Worktree kept.
