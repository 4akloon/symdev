# T1-shadow: `UidCrc` type + Wine stdout compare

Date: 2026-09-17
Status: approved for SDD (user: prefer OOP, fewer free functions; Wine stdout is CRLF).

Cites: [2026-09-17-symdev-t1-uidcrc-design.md](2026-09-17-symdev-t1-uidcrc-design.md); experiment 13.

## 1. Goal

Replace the public free-function `uidcrc` surface with a `UidCrc` value type (same goldens and argv as T1). Add a method that decides whether Wine `uidcrc.exe` stdout matches `UidCrc::line`, after stripping CR/LF. Default tests never spawn Wine. Still no clap verb and no `package` wiring.

## 2. Locked decisions

| # | Decision |
|---|---|
| Shape | Public API is `UidCrc { uid1, uid2, uid3 }` with methods. Do not keep `uid_checked` / `uidcrc_bytes` / `uidcrc_line` / `uidcrc_args` as public free functions. Private CRC helper may stay a function. |
| Compare | UTF-8 lossy stdout, trim ASCII whitespace including `\r` and `\n`, then `== self.line()`. |
| Wine | Not spawned in default `cargo test`. No `SYMDEV_SHADOW` env in this slice (no call site on the Wave 0 path). |
| Argv | Same recorded tokens as T1 `uidcrc_args`. Do not invent flags. |
| Fail | `matches_wine` returns `bool`. Do not log passwords (uidcrc has none). |
| Going forward | New public APIs in subsequent slices are types + methods unless a free function is the constructor of that type (`parse_*` returning a struct is fine). |

## 3. Interfaces

```rust
pub struct UidCrc {
    pub uid1: u32,
    pub uid2: u32,
    pub uid3: u32,
}

impl UidCrc {
    pub fn new(uid1: u32, uid2: u32, uid3: u32) -> Self;
    pub fn checked(&self) -> u32;
    pub fn bytes(&self) -> [u8; 16];
    pub fn line(&self) -> String; // "0x%08x 0x%08x 0x%08x 0x%08x" lowercase
    pub fn wine_args(&self, wine: &Path, uidcrc: &Path, outfile: Option<&str>) -> Vec<String>;
    pub fn normalize_stdout(bytes: &[u8]) -> String;
    pub fn matches_wine(&self, wine_stdout: &[u8]) -> bool;
}
```

`normalize_stdout`: `String::from_utf8_lossy(bytes)` then `.trim_matches(['\r', '\n', ' ', '\t'])` — experiment 13 is CRLF after the four hex tokens. Trim only leading/trailing CR/LF/space/tab, not internal spaces.

`matches_wine`: `Self::normalize_stdout(wine_stdout) == self.line()`.

Pinned: bytes of `0x1000007a 0x100039ce 0xe79e4cf9 0x5dcf194e\r\n` match hello UIDs. Same tokens with only `\n` also match. A wrong checked UID does not.

Goldens and argv from T1 stay identical; only the call shape changes (`UidCrc::new(u1, u2, u3).checked()` etc.).

## 4. Non-goals

Native makekeys; spawning Wine in CI; clap; calling this from `symdev package`; `SYMDEV_SHADOW` orchestration.
