# T1 clean-room `uidcrc` (shadow later)

Date: 2026-09-17
Status: approved for implementation (user: continue the plan, commit and push).

Cites: north-star §18 T-track; [experiment-backlog.md](../../research/experiment-backlog.md) experiments 9 and 13; [licensing.md](../../research/licensing.md).

## 1. Goal

Ship a **clean-room** checked-UID function that matches Wine `uidcrc.exe` goldens recorded in experiment 13. Default `cargo test` does not spawn Wine. No new clap command.

## 2. Why this, not M3 / M5 / native makekeys

- M3 SSH needs a macOS edit host (skipped).
- M5 / experiment 10: EKA2L1 CLI is Unknown without `SYMDEV_EKA2L1` + `SYMDEV_ROM` (still unset). Do not invent emulator flags.
- Deploy copy/OBEX needs a phone (experiment 11 skip).
- T-track: “T1 with M2 (`uidcrc`, `makekeys`, shadow)” once M1+ exists. M1 and M2 are on `main`.
- Native `makekeys` cannot byte-compare with Wine (RSA material + certificate dates). **Out of this slice.** Wine `makekeys` stays the signer. Shadow-for-makekeys waits until a normalize rule exists.

## 3. Locked decisions

| # | Decision |
|---|---|
| Surface | Library in `symdev-build` (`uidcrc.rs`). Not a `symdev uidcrc` clap verb. |
| Sources | Do not copy `uidcrc` C / Symbian Example Source. Goldens + the CRC step recorded in experiment 13. |
| Argv | Wine: `wine <uidcrc.exe> <uid1> <uid2> <uid3> [<outfile>]` as recorded. Hex `0x` + digits. |
| Tests | Pinned triples from experiment 13. No Wine in default tests. |
| Package | `symdev package` still uses Wine SIS tools. Do not call `uidcrc` from the package path in this slice (hello has no `.rsc`). |
| Secrets | Never commit `.cer` / `.key` / `.sis` / `.sisx`. |

## 4. Algorithm (from experiment 13 goldens)

Let `uid1, uid2, uid3: u32`.

1. Twelve bytes: little-endian `uid1` then `uid2` then `uid3`.
2. Even bytes = indices 0,2,4,6,8,10. Odd bytes = 1,3,5,7,9,11.
3. CRC16 (16-bit, init 0) per byte `b`:
   - `crc = rotl8(crc) XOR b`
   - `crc ^= (crc AND 0xff) >> 4`
   - `crc ^= crc << 12`
   - `crc ^= (crc AND 0xff) << 5`
   (`rotl8` on `u16` is `(crc << 8) | (crc >> 8)`.)
4. `checked = (crc16(odd) as u32) << 16 | (crc16(even) as u32)`

Pinned: `(0x1000007a, 0x100039ce, 0xe79e4cf9)` → `0x5dcf194e`.

## 5. Interfaces

Superseded by [2026-09-17-symdev-t1-shadow-design.md](2026-09-17-symdev-t1-shadow-design.md): public API is `UidCrc` methods (`checked`, `bytes`, `line`, `wine_args`). Goldens and argv below are unchanged.

```rust
pub struct UidCrc { pub uid1: u32, pub uid2: u32, pub uid3: u32 }
impl UidCrc {
    pub fn new(uid1: u32, uid2: u32, uid3: u32) -> Self;
    pub fn checked(&self) -> u32;
    pub fn bytes(&self) -> [u8; 16]; // four LE u32
    pub fn line(&self) -> String; // "0x%08x 0x%08x 0x%08x 0x%08x" lowercase
    pub fn wine_args(&self, wine: &Path, uidcrc: &Path, outfile: Option<&str>) -> Vec<String>;
}
```

`wine_args` without outfile is the stdout form; with outfile appends that filename (relative, like SIS argv).

## 6. Non-goals

Native makekeys; shadow subprocess in default tests; clap `uidcrc`; rcomp; `_reg.rss`; E52 claim; M5.

## 7. Testing

Goldens from experiment 13 (all six stdout triples). `uidcrc_bytes` for the hello triple matches the 16 recorded bytes. `uidcrc_args` pins `/usr/bin/wine` + `/sdk/epoc32/tools/uidcrc.exe` + `0x1000007a` `0x100039ce` `0xe79e4cf9` `out.uid`.
