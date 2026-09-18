# T4 elf2e32 — native uncompressed E32 image header

Date: 2026-09-18
Branch: `t4-e32-header`
Worktree: `/home/genius/worktrees/symdev/t4-e32-header` (kept)
Primary checkout: `/home/genius/projects/symdev` still `main` (untouched)
Base: `origin/main` `0d684151079f5263418d5dbffd289c08ac188ee7`

## Result

**All 156 uncompressed header bytes of frozen experiment-6 `hello.exe` matched.** `E32ImageHeader::uncompressed` (this + `E32ImageHeaderJ` + `E32ImageHeaderV`) equals `golden[..156]`, including `iHeaderCrc` `0xda38d643` and the frozen Symbian time. Disk file `$HOME/src/symdev-experiment-5/hello.exe` is byte-identical to in-crate hex.

`Elf2E32::encode` and the `elf2e32` bin stay `TODO`. No ELF parse. No deflate of the 3432-byte payload. Wave 0 `GcceBuild` still spawns Linux `elf2e32_next` via `SYMDEV_ELF2E32`. No Wine. Domain types do not know Wine. No clap verb. Never `-fPIC`. No E52. No Rust-app std target (track B).

## SHAs

| What | SHA |
|---|---|
| origin/main base | `0d684151079f5263418d5dbffd289c08ac188ee7` |
| Native header encode | `cbeda2ab2d6a9b6e7ba713bbb550d77ba83f0e3d` |
| Frozen `hello.exe` | SHA-256 `1a25bc1dbc2c5aaed1333e8c606d0d3c6ad6ed5cc2ffd836e6b1a3e42c65b50b` (3588 bytes) |

Feature-branch tip SHA is the report commit on this branch.

## Header layout (little-endian; `iCodeOffset` = 156)

`E32ImageHeader` 124 + `E32ImageHeaderJ` 4 + `E32ImageHeaderV` 28 (includes `iExportDesc[1]` pad). Present on this golden as header format V.

| Off | Size | Field | Golden | Proven how |
|---|---|---|---|---|
| 0x00 | 16 | `E32Uid` (`iUid1/2/3` + checksum) | `7a00001000000000f94c9ee7b00832c1` | previous slice; `for_exe` |
| 0x10 | 4 | `iSignature` | `EPOC` | constant |
| 0x14 | 4 | `iHeaderCrc` | `0xda38d643` | CRC-32 of 156 bytes with slot = `0xc90fdaa2` |
| 0x18 | 4 | `iModuleVersion` | `0x000a0000` | `--linkas=hello{000a0000}[…]` |
| 0x1c | 4 | `iCompressionType` | `0x101f7afc` | deflate default (not on argv) |
| 0x20 | 4 | `ToolVersion` 3.0.2 | `03 00 02 00` | elf2e32_next 3.0 Build 2 |
| 0x24 | 4 | `iTimeLo` | `0x208d5e00` | frozen golden (live on replay) |
| 0x28 | 4 | `iTimeHi` | `0x00e33963` | frozen golden (live on replay) |
| 0x2c | 4 | `iFlags` | `0x1200002a` | `--fpu=softvfp` + EXE defaults (ELF import \| hdr V \| EKA2 \| EABI \| no-call-entry) |
| 0x30 | 4 | `iCodeSize` | `0x144c` (5196) | ELF-derived; pinned from golden |
| 0x34 | 4 | `iDataSize` | `0` | ELF-derived; pinned |
| 0x38 | 4 | `iHeapSizeMin` | `0x1000` | default |
| 0x3c | 4 | `iHeapSizeMax` | `0x100000` | default |
| 0x40 | 4 | `iStackSize` | `0x2000` | default |
| 0x44 | 4 | `iBssSize` | `4` | ELF-derived; pinned |
| 0x48 | 4 | `iEntryPoint` | `0x1098` | ELF-derived; pinned |
| 0x4c | 4 | `iCodeBase` | `0x8000` | ELF / recorded `-Ttext 0x8000`; pinned |
| 0x50 | 4 | `iDataBase` | `0x400000` | ELF-derived; pinned |
| 0x54 | 4 | `iDllRefTableCount` | `2` | ELF-derived; pinned |
| 0x58 | 4 | `iExportDirOffset` | `0` | pinned |
| 0x5c | 4 | `iExportDirCount` | `0` | pinned |
| 0x60 | 4 | `iTextSize` | `0x144c` | ELF-derived; pinned |
| 0x64 | 4 | `iCodeOffset` | `0x9c` (156) | sizeof header+J+V |
| 0x68 | 4 | `iDataOffset` | `0` | pinned |
| 0x6c | 4 | `iImportOffset` | `0x14e8` | ELF-derived; pinned |
| 0x70 | 4 | `iCodeRelocOffset` | `0x15b8` | ELF-derived; pinned |
| 0x74 | 4 | `iDataRelocOffset` | `0` | pinned |
| 0x78 | 2 | `iProcessPriority` | 350 | foreground default |
| 0x7a | 2 | `iCpuIdentifier` | `0x2001` | ARMv5 default |
| 0x7c | 4 | J `iUncompressedSize` | `0x1578` (5496) | layout/ELF; pinned |
| 0x80 | 4 | V `iSecureId` | `0xe79e4cf9` | SID = uid3 (`--sid` not on argv) |
| 0x84 | 4 | V `iVendorId` | `0` | `--vid` not on argv |
| 0x88 | 8 | V `iCaps` | `0xbe000` | six recorded names; word pinned, packing not re-derived here |
| 0x90 | 4 | V `iExceptionDescriptor` | `0x10f5` | ELF-derived; pinned |
| 0x94 | 4 | V `iSpare2` | `0` | constant |
| 0x98 | 2 | V `iExportDescSize` | `0` | no bitmap |
| 0x9a | 1 | V `iExportDescType` | `0x01` | FullBitmap (elf2e32_next EXE) |
| 0x9b | 1 | V `iExportDesc[0]` pad | `0` | size 0 |

Bytes after 0x9c (3432) are deflate of the uncompressed image body. Not this slice.

## What matched

The whole uncompressed header. CRC algorithm matches elf2e32_next (`Crc32` init 0, reflected poly `0xedb88320`, `iHeaderCrc` slot held at `KImageCrcInitialiser` `0xc90fdaa2` during the pass). Frozen timestamps are reproduced from the golden, not generated.

Suffix after the header is still unknown to native encode (compressed payload). No unknown suffix *inside* the 156-byte header.

## Types

`E32ImageHeader`, `E32ImageHeaderJ`, `E32ImageHeaderV` in `crates/symdev-elf2e32/src/e32.rs`. `bytes` on each piece; `E32ImageHeader::uncompressed` concatenates and stamps CRC. Crate-root `pub use` append-only. `Elf2E32Tool` stays Linux-path argv, not Wine.

## Tests

TDD: types missing (E0425/E0433), then 7 passed.

`cargo test --workspace --offline` (after GREEN):

- `symdev-build` 40 passed
- `symdev` unittests 3 passed
- `cli` 19 passed
- `symdev-core` 5 passed
- `symdev-elf2e32` 7 passed (2 new: `EPOC` after UID; 156-byte uncompressed header)
- `symdev-makekeys` 3 passed
- `symdev-manifest` 17 passed
- `symdev-rcomp` 15 passed
- `symdev-sis` 72 passed
- `symdev-uidcrc` 9 passed

**190 tests, 0 failed.** No Wine. No SDK / `.elf` / `.exe` binaries / `.cer` / `.key` committed.

## Remaining (next T4 slices)

- ELF parse of `hello.elf` → fill the pinned ELF-derived fields instead of literals
- Capability bit packing from the recorded six names (`iCaps = 0xbe000` is pinned; SIS already maps those names)
- Deflate of the post-header payload (3432 bytes compressed; uncompressed `0x1578`)
- `Elf2E32::encode` / bin writing a real E32; wiring native encode into `GcceBuild` (still `SYMDEV_ELF2E32`)
- `--libpath` DSO consumption; variable `iExportDescSize`; non-EXE `targettype` UID2; `--uid2` if ever observed
- Live `iTimeLo`/`iTimeHi` (structure pinned; golden timestamps used)
- Track B (Rust-app std target) is out of scope

No merge to `main`. Worktree kept.
