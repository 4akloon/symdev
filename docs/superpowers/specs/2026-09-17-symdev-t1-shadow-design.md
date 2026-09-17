# T1-shadow: `UidCrc` value + `UidCrcTool` Wine compare

Date: 2026-09-17
Status: approved for SDD; 2026-09-17 follow-up: `UidCrc` must not know about Wine.

Cites: [2026-09-17-symdev-t1-uidcrc-design.md](2026-09-17-symdev-t1-uidcrc-design.md); experiment 13.

## 1. Goal

Public `uidcrc` API is two types: `UidCrc` (three UIDs + checked CRC) and `UidCrcTool` (Wine + `uidcrc.exe` paths, recorded argv, stdout compare). Same goldens and argv as T1. Default tests never spawn Wine. Still no clap verb and no `package` wiring.

## 2. Locked decisions

| # | Decision |
|---|---|
| Shape | `UidCrc` is the value. `UidCrcTool` is the host process (same split as `SisTools`). Do not put Wine paths, argv, or stdout compare on `UidCrc`. Do not keep public free functions. Private CRC helper may stay a function. |
| Compare | UTF-8 lossy stdout, trim ASCII whitespace including `\r` and `\n`, then `== crc.line()`. Lives on `UidCrcTool`. |
| Wine | Not spawned in default `cargo test`. No `SYMDEV_SHADOW` env in this slice (no call site on the Wave 0 path). |
| Argv | Same recorded tokens as T1. Do not invent flags. Built by `UidCrcTool::args`. |
| Fail | `UidCrcTool::output_matches` returns `bool`. Do not log passwords (uidcrc has none). |
| Going forward | New public APIs in subsequent slices are types + methods unless a free function is the constructor of that type (`parse_*` returning a struct is fine). A value type does not take host-tool paths. |

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
}

pub struct UidCrcTool {
    pub wine: PathBuf,
    pub uidcrc: PathBuf,
}

impl UidCrcTool {
    pub fn new(wine: &Path, uidcrc: &Path) -> Self;
    pub fn args(&self, crc: &UidCrc, outfile: Option<&str>) -> Vec<String>;
    pub fn normalize_stdout(bytes: &[u8]) -> String;
    pub fn output_matches(crc: &UidCrc, stdout: &[u8]) -> bool;
}
```

`normalize_stdout`: `String::from_utf8_lossy(bytes)` then `.trim_matches(['\r', '\n', ' ', '\t'])` — experiment 13 is CRLF after the four hex tokens. Trim only leading/trailing CR/LF/space/tab, not internal spaces.

`output_matches`: `Self::normalize_stdout(stdout) == crc.line()`.

Pinned: bytes of `0x1000007a 0x100039ce 0xe79e4cf9 0x5dcf194e\r\n` match hello UIDs. Same tokens with only `\n` also match. A wrong checked UID does not.

Goldens and argv from T1 stay identical; only the call shape changes (`UidCrc::new(...).checked()`, `tool.args(&crc, ...)`).

## 4. Non-goals

Native makekeys; spawning Wine in CI; clap; calling this from `symdev package`; `SYMDEV_SHADOW` orchestration.
