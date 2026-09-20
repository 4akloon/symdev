# symdev-manifest

Parser and validator for `symdev.toml`, the project manifest read by every `symdev` command.
Unknown keys are rejected, and every error names the offending key.

```rust
let manifest = symdev_manifest::load("symdev.toml")?;
let manifest = symdev_manifest::parse(src)?;       // from a string
```

## Format

```toml
[package]
name = "hello"          # starts with a letter; letters, digits, underscore
version = "0.1.0"

[target]
device = "nokia-e52"

[language]
name = "cpp"

[symbian]
uid3 = "0xef9f2cab"     # 0x + 8 hex digits, in 0xA0000000..=0xAFFFFFFF or 0xE0000000..=0xEFFFFFFF
capabilities = []       # user-grantable only
vendor = "symdev"
icon = "gfx/hello.svg"  # optional; built into <app>_aif.mif

[signing]
mode = "self-signed"
# cert = "keys/dev.cer"; key = "keys/dev.key"; subject = "CN=Vendor,O=Vendor"  (all optional)

[[install]]             # extra files carried in the package
source = "build/games.mbm"
dest = "\\resource\\apps\\games.mbm"

[[icons]]               # one mifconv-style icon container per entry
dest = "\\resource\\apps\\games.mif"
header = "games.mbg"
depth = "c32,8"
sources = ["gfx/a.svg", { file = "gfx/b.svg", depth = "c24" }]
```

`platform` and `toolchain` sections are optional; the compiler may be `gcce-14` or `gcce-15`.

- **Capabilities**: only the user-grantable set (`USER_GRANTABLE`: `LocalServices`,
  `NetworkServices`, `ReadUserData`, `WriteUserData`, `UserEnvironment`, `Location`) is
  accepted, because the only signing mode is self-signed. A privileged or unknown capability, and
  a duplicate, is an error.
- **`[[install]]`** destinations are normalised to `!:\…` (the drive the user picks).
- **`[[icons]]`** entries must each build a distinct file name and header.

## Public types

`Manifest` with `Package`, `Target`, `Platform`, `Language`, `Toolchain`, `Symbian`, `Signing`,
`InstallFile`, `IconContainer`, `IconSource`, plus `Error` and `Result`.

## Testing

```bash
cargo test -p symdev-manifest --offline
```
