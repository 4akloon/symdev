# T2: SIS UTF-16 string (`SisString`)

Date: 2026-09-18
Status: approved for SDD (user: continue T-track; types + methods; value types do not know Wine).

Cites: [2026-09-18-symdev-t2-sis-field-design.md](2026-09-18-symdev-t2-sis-field-design.md); [experiment-backlog.md](../../research/experiment-backlog.md) experiment 17.

## 1. Goal

A `SisString` value type that encodes a SIS type-1 string as UTF-16-LE with no BOM and no NUL, then wraps as a `SisField`. Default tests never spawn Wine, never inflate, and never read `.sis` from disk. Still no clap verb. `symdev package` still uses Wine `SisTools`.

## 2. Why this, not inflate / arrays

Experiment 17 pins the type-1 payloads from a host-side inflate of hello.sis. In-tree inflate, type-2 arrays, and type-4 version stay later slices. `SisField` already pads odd UTF-16 byte lengths.

## 3. Locked decisions

| # | Decision |
|---|---|
| Shape | `SisString { text }` with methods. No Wine paths. `SisTools` stays the host process. |
| Encoding | UTF-16-LE, no BOM, no terminating NUL. Use `str::encode_utf16`. No extra crates. |
| Field | `field()` returns `SisField::new(Self::KIND, self.payload())` with `KIND = 1`. Padding is `SisField`’s job. |
| Tests | Pinned `Vendor` payload and `hello` field bytes from experiment 17. No Wine. Do not commit or read `.sis` / `.sisx`. |
| Package | Do not call `SisString` from `SisPackage` in this slice. |
| Sources | Do not copy MakeSIS C. Goldens only. |

## 4. Interfaces

```rust
pub struct SisString {
    pub text: String,
}

impl SisString {
    pub const KIND: u32 = 1;

    pub fn new(text: impl Into<String>) -> Self;
    pub fn payload(&self) -> Vec<u8>;
    pub fn field(&self) -> SisField;
}
```

`payload`: concatenate each `encode_utf16` unit as little-endian `u16`.

`field`: `SisField::new(Self::KIND, self.payload())`.

Pinned:

- `SisString::new("Vendor").payload()` equals `56 00 65 00 6e 00 64 00 6f 00 72 00`.
- `SisString::new("hello").field().bytes()` equals `01 00 00 00 0a 00 00 00 68 00 65 00 6c 00 6c 00 6f 00 00 00` (length 10, two pad bytes).

## 5. Non-goals

In-tree inflate; type-2 arrays; type-4 version; native `signsis` / `makekeys`; wiring into `package`; clap; spawning Wine; E52 claim; new crates.
