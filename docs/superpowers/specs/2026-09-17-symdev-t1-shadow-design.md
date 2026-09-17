# T1-shadow: compare native `uidcrc` to Wine stdout

Date: 2026-09-17
Status: approved for SDD (continue T-track; Wine stdout is CRLF).

Cites: [2026-09-17-symdev-t1-uidcrc-design.md](2026-09-17-symdev-t1-uidcrc-design.md); experiment 13.

## 1. Goal

A helper that decides whether Wine `uidcrc.exe` stdout matches `uidcrc_line`, after stripping CR/LF. Default tests never spawn Wine. Still no clap verb and no `package` wiring.

## 2. Locked decisions

| # | Decision |
|---|---|
| Compare | UTF-8 lossy stdout, trim ASCII whitespace including `\r` and `\n`, then `== uidcrc_line(...)` |
| Wine | Not spawned in default `cargo test`. No `SYMDEV_SHADOW` env in this slice (no call site on the Wave 0 path). |
| Argv | Reuse `uidcrc_args`. Do not invent flags. |
| Fail | `uidcrc_matches_wine` returns `bool`. Do not log passwords (uidcrc has none). |

## 3. Interfaces

```rust
pub fn normalize_uidcrc_stdout(bytes: &[u8]) -> String;
pub fn uidcrc_matches_wine(uid1: u32, uid2: u32, uid3: u32, wine_stdout: &[u8]) -> bool;
```

`normalize_uidcrc_stdout`: `String::from_utf8_lossy(bytes)` then `.trim_matches(|c: char| c == '\r' || c == '\n' || c == ' ' || c == '\t')` — experiment 13 is CRLF after the four hex tokens. Trim only leading/trailing CR/LF/space/tab, not internal spaces.

Pinned: bytes of `0x1000007a 0x100039ce 0xe79e4cf9 0x5dcf194e\r\n` match hello UIDs. Same tokens with only `\n` also match. A wrong checked UID does not.

## 4. Non-goals

Native makekeys; spawning Wine in CI; clap; calling this from `symdev package`; `SYMDEV_SHADOW` orchestration.
