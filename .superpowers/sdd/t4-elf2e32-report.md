# T4 elf2e32 — native E32 UID encode from Linux `elf2e32_next`

Date: 2026-09-18
Branch: `t4-elf2e32`
Worktree: `/home/genius/worktrees/symdev/t4-elf2e32` (kept)
Primary checkout: `/home/genius/projects/symdev` still `main` (untouched)
Base: `origin/main` `7f176303b48caf59b48681419d2f2e874a7e7922`

## Result

**16-byte UID prefix of frozen experiment-6 `hello.exe` matched.** `E32Uid::for_exe(0x1000007a, 0xe79e4cf9).bytes()` equals the first 16 bytes of Linux `elf2e32_next` 3.0 Build 2 output. `Elf2E32::uid()` maps recorded experiment-6 argv onto that type.

`--uid2` was **not** on the recorded argv (`GcceBuild::elf2e32_args` still omits it). The image has `iUid2 = 0`, checksum `0xc13208b0`. Do not invent `--uid2` or copy SIS `UidCrc` hello’s `0x100039ce`.

Full 3588-byte file is pinned as hex. Native encode covers the UID header only. `Elf2E32::encode` and the `elf2e32` bin stay `TODO`. Wave 0 `GcceBuild` still spawns Linux `elf2e32_next` via `SYMDEV_ELF2E32`. No Wine. Domain type does not know Wine. No clap verb. Never `-fPIC`. No E52. No Rust-app std target (track B).

Replay of recorded experiment-6 argv on the frozen `hello.elf` reproduced the same 16-byte UID prefix; the rest of the file differed only at `iHeaderCrc` and Symbian time (8 bytes). Payload after the header is deterministic for this ELF.

## SHAs

| What | SHA |
|---|---|
| origin/main base | `7f176303b48caf59b48681419d2f2e874a7e7922` |
| Native `E32Uid` + experiment-6 hex | `336951948ff0cd429777d161ba9aea3980b091c0` |
| Frozen `hello.exe` | SHA-256 `1a25bc1dbc2c5aaed1333e8c606d0d3c6ad6ed5cc2ffd836e6b1a3e42c65b50b` (3588 bytes) |

Feature-branch tip SHA is the report commit on this branch.

## Recorded argv (experiment 6; not invented)

```
/home/genius/src/elf2e32_next/bin/Release/elf2e32 \
  --uid1=0x1000007a --uid3=0xe79e4cf9 \
  --capability=LocalServices+NetworkServices+ReadUserData+WriteUserData+UserEnvironment+Location \
  --fpu=softvfp --targettype=EXE --output=hello.exe --elfinput=hello.elf \
  --linkas=hello{000a0000}[e79e4cf9].exe \
  --libpath=/home/genius/sdk/S60_3rd_FP2/epoc32/release/armv5/lib
```

Wine/PE `elf2e32.exe` is not the product path.

## Pinned golden

Frozen experiment-6 E32 at `$HOME/src/symdev-experiment-5/hello.exe` (outside git). In-crate hex: `crates/symdev-elf2e32/src/testdata/hello.exe.hex`.

| File | Header (16 bytes) | UID1 | UID2 | UID3 | checked |
|---|---|---|---|---|---|
| `hello.exe` | `7a00001000000000f94c9ee7b00832c1` | `0x1000007a` | `0x00000000` | `0xe79e4cf9` | `0xc13208b0` |

Observed header after the UID (not produced this slice): signature `EPOC`; `iModuleVersion` `0x000a0000` from `--linkas`; deflate `iCompressionType` `0x101f7afc`; tool `3.0.2`; `iCodeOffset` `0x9c` (156-byte header); `iCpuIdentifier` ARMv5 `0x2001`; `iProcessPriority` 350; SID = uid3; VID 0; `iCaps` `0xbe000`; heap/stack defaults `0x1000`/`0x100000`/`0x2000`.

## Types

`E32Uid` in `crates/symdev-elf2e32/src/e32.rs`. `for_exe`, `crc` (`UidCrc`), `bytes`. `EXE_UID2` is `0` (omitted `--uid2`). Crate-root `pub use e32::E32Uid` (append-only). `Elf2E32::uid()` → `E32Uid::for_exe(uid1, uid3)`. `Elf2E32Tool` stays Linux-path argv, not Wine.

## Tests

TDD: `E32Uid` missing (E0433), then `Elf2E32::uid` missing (E0599), then 5 passed.

`cargo test --workspace --offline` (after GREEN):

- `symdev-build` 40 passed
- `symdev` unittests 3 passed
- `cli` 19 passed
- `symdev-core` 5 passed
- `symdev-elf2e32` 5 passed (3 new: UID bytes, checked, argv → prefix)
- `symdev-makekeys` 3 passed
- `symdev-manifest` 17 passed
- `symdev-rcomp` 15 passed
- `symdev-sis` 72 passed
- `symdev-uidcrc` 9 passed

**188 tests, 0 failed.** No Wine. No SDK / `.elf` / `.exe` binaries / `.cer` / `.key` committed.

## Remaining unknowns (next T4 slices)

- Full `E32ImageHeader` + J + V encode (flags `0x1200002a`, export-desc type `0x01` with size 0, exception descriptor `0x10f5`)
- Header CRC (`iHeaderCrc`) and Symbian `iTimeLo`/`iTimeHi` (replay is not byte-identical)
- ELF parse of `hello.elf` → code/data/bss/entry/imports (`dllRefTableCount` 2)/relocs
- Deflate of the post-header payload (3432 bytes compressed; uncompressed `0x1578`)
- Capability bit packing from the recorded six names (`iCaps = 0xbe000`)
- `Elf2E32::encode` / bin writing a real E32; wiring native encode into `GcceBuild` (still `SYMDEV_ELF2E32`)
- `--libpath` DSO consumption; non-EXE `targettype` UID2; `--uid2` if ever observed
- Track B (Rust-app std target) is out of scope

No merge to `main`. Worktree kept.
