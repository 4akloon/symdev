# Ported SDK tools as crates

User decision: **split** each ported SDK PE into its own `symdev-*` crate, with a CLI bin when the Windows PE was a CLI. Reproduces Wave 0 / Nokia E52 functionality this project needs. Flags needed later stay `// TODO` / `todo!` — no fake encode.

`symdev-build` stays the orchestrator (MMP, `GcceBuild` driver, `SisPackage` flow) and depends on the tool crates. Product CLI `symdev` uses libraries; it does not spawn the tool bins.

Locked facts this note does not reopen: Linux host; Wave 0 GCC/`ld` stay external; self-sign only; types + methods (not free-function piles); Wine/argv only on `*Tool`; recorded SDK argv only; never commit `.sis` / `.cer` / `.key`.

## Crate map

Workspace members, one version (`0.1.0`), resolver `"3"`, edition 2024, rust-version 1.98.1.

| PE | crate | bin | Wave 0 role |
|---|---|---|---|
| uidcrc.exe | `symdev-uidcrc` | `uidcrc` | `UidCrc` / `UidCrcTool`; native bin writes 16-byte file or prints the stdout line |
| makesis.exe | `symdev-sis` | `makesis` | SIS encode types + `Makesis` (`-v pkg sis`); Wave 0 one-file `TYPE=SA` `.pkg` |
| signsis.exe | `symdev-sis` | `signsis` | `SisUnsigned::encode_signed` on the library; bin parses recorded positionals then `todo!` inflate |
| makekeys.exe | `symdev-makekeys` | `makekeys` | `SelfSignedDsa` + `MakekeysTool` (Wine argv) + `Makekeys` (`-cert` recorded tokens) |
| rcomp.exe | `symdev-rcomp` | `rcomp` | `Rsc::bytes()` matches Wine goldens (`Rsc`, `RscAppRegistration`, `RscLtext16`, `RscResource`); bin parses `-u -o -s -i [-h]` then `todo!` RSS source / `.rsg` |
| elf2e32.exe | `symdev-elf2e32` | `elf2e32` | thin `Elf2E32` from recorded `--key=value`; `todo!` encode, no fake E32 |
| mifconv.exe / bmconv.exe | — | — | **TODO only** — no crate, no fake impl |

Also: `symdev-core`, `symdev-manifest`, `symdev-build` (orchestrator), `symdev-cli` (`symdev`).

DAG:

```
symdev-uidcrc  ←  symdev-rcomp
symdev-uidcrc  ←  symdev-sis   (SisUid + checksums via epoc_crc16)
symdev-sis     ←  SisUnsigned encode / Makesis / Signsis
symdev-makekeys
symdev-build   ←  MMP/driver + SisPackage (uses sis + makekeys)
symdev-cli     ←  GcceBuild + SisPackage + Toolchain
```

## Types vs adapters

| Crate | Value types | `*Tool` / native CLI type |
|---|---|---|
| `symdev-uidcrc` | `UidCrc` | `UidCrcTool` (Wine) |
| `symdev-sis` | `Sis*` encode types | `SisTools` (Wine makesis/signsis); `Makesis` / `Signsis` (native argv) |
| `symdev-makekeys` | `SelfSignedDsa` | `MakekeysTool` (Wine); `Makekeys` (native `-cert`) |
| `symdev-rcomp` | `RscUid`, `Rsc`, `RscAppRegistration`, `RscLtext16`, `RscResource` | `RcompTool` (Wine); `Rcomp` (native flags) |
| `symdev-elf2e32` | `Elf2E32` (recorded flags) | bin `todo!` encode |
| `symdev-build` | `GcceBuild`, `SisPackage`, `Mmp`, `BldInf` | `Toolchain::from_env` |

`SelfSignedDsa` stays on makekeys. `UidCrc` vs `UidCrcTool` split remains. `SisPackage::pkg_text` / `Mmp::parse` / `BldInf::parse` stay methods — do not resurrect `render_pkg` / `parse_mmp` / `parse_bld_inf`.

## TODOs (not Nokia E52 Wave 0)

Leave these unimplemented (`// TODO` / `todo!` / `Err(Error::Other("TODO: …"))`):

- **makesis:** `-h -i -s -d directory`; pkg grammar beyond one-file `TYPE=SA` `&EN` + platform UID `0x102752AE`; capability bits from E32 (Wine makesis; not in Wave 0 `.pkg`)
- **signsis:** inflate existing unsigned SIS + attach signatures; flags `-? -h -c* -i -o -p -s -u -v`
- **makekeys:** `-req` / `-view`; `-expdays` other than 3650; `-len` other than recorded 2048 token (native still generates DSA 1024/160); `-dname` other than the recorded example
- **rcomp:** RSS source parse, `.rsg` emission, unused usage flags `-v -p -l -force -{uid2,uid3}`
- **elf2e32:** native ELF→E32 encode (Wave 0 still spawns the Linux C++ binary via `GcceBuild`)
- **mifconv / bmconv:** no crate

Default `cargo test` does not spawn Wine.
