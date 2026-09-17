# T2 first slice: SIS UID header (`SisUid`)

Date: 2026-09-17
Status: approved for SDD (user: continue T-track; types + methods; value types do not know Wine).

Cites: north-star §18 T-track T2; [experiment-backlog.md](../../research/experiment-backlog.md) experiment 14; [2026-09-17-symdev-t1-shadow-design.md](2026-09-17-symdev-t1-shadow-design.md).

## 1. Goal

A `SisUid` value type whose 16-byte header matches Wine `makesis` / `signsis` goldens for hello. Reuse `UidCrc`. Default tests never spawn Wine and never read `.sis` from disk. Still no clap verb. `symdev package` still uses Wine `SisTools`.

## 2. Why this, not full native makesis

- T2 is `makesis`/`signsis` + golden. The frozen experiment-7/8 files already pin the UID block. The rest of the SIS (controller field, compression, signatures, creation time) is a later slice — north-star says normalize SIS creation time / signatures / checksums / padding before byte-compare.
- Native `makekeys` is still not golden-diffable (RSA + dates).
- M3/M5/E52 remain skipped.

## 3. Locked decisions

| # | Decision |
|---|---|
| Shape | `SisUid { package }` with methods. SIS UID1/UID2 are associated constants. Checksum is `UidCrc::new(UID1, UID2, package)`. No Wine paths on `SisUid`. `SisTools` stays the host process. |
| UID1 | `0x10201a7a` as recorded on hello.sis |
| UID2 | `0` as recorded (do not substitute a wiki UID2) |
| UID3 | Package UID (hello `0xe79e4cf9`) |
| Tests | Pinned 16 raw bytes from experiment 14. No Wine. Do not commit or read `.sis` / `.sisx`. |
| Package | Do not call `SisUid` from `SisPackage` in this slice. |
| Sources | Do not copy MakeSIS C. Goldens only. |

## 4. Interfaces

```rust
pub struct SisUid {
    pub package: u32,
}

impl SisUid {
    pub const UID1: u32 = 0x1020_1a7a;
    pub const UID2: u32 = 0;

    pub fn new(package: u32) -> Self;
    pub fn crc(&self) -> UidCrc;
    pub fn bytes(&self) -> [u8; 16];
}
```

`crc()` is `UidCrc::new(Self::UID1, Self::UID2, self.package)`.
`bytes()` is `self.crc().bytes()`.

Pinned: `SisUid::new(0xe79e_4cf9).bytes()` equals
`7a 1a 20 10 00 00 00 00 f9 4c 9e e7 04 00 b4 5d`.
`crc().checked()` equals `0x5db4_0004`.

## 5. Non-goals

Native SIS body; native `signsis`; native `makekeys`; wiring into `package`; clap; spawning Wine; E52 claim.
