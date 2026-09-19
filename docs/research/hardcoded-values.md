# Hardcoded values: what is configurable, and what must not be

Investigation only. No config system, no refactor. Primary checkout `origin/main`. Facts are **Verified** against in-tree types/methods unless labeled **Unknown**.

Cites: [uids-capabilities-signing.md](uids-capabilities-signing.md); [pipeline-and-tools.md](pipeline-and-tools.md); [experiment-backlog.md](experiment-backlog.md) experiments 5–8, 13–41; crates `symdev-manifest`, `symdev-build` (`sis/`, `pkg.rs`, `driver.rs`, `toolchain.rs`, `uidcrc.rs`, `rcomp/`), `symdev-cli`.

Locked product decisions this note does **not** reopen: Linux host edit+build; Wave 0 GCC; no `-fPIC`; self-sign only; platform UID `0x102752AE`; test UIDs `0xE…`; six user-grantable capabilities; never invent argv; never claim E52 until stock install.

## Executive answer

Not every literal in the tree is a missing `symdev.toml` key. Native SIS/RSC encode was reverse-engineered against **one frozen hello golden** (`makesis` / `signsis` / `rcomp` from experiments 7–8 and 41). Some numbers are the file format (change them and the phone rejects the SIS). Some are already on `Manifest` / env. Some are hello-app identity that a real author would set, still baked because T2 had to byte-match Wine. Some are toolchain internals (Wine adapter, recorded SignSIS `k`, zlib level) that look like knobs but are not product settings.

The short why: **configurability without a grammar is how you ship an uninstallable SIS.** MakeSIS option words, SIS field kinds, and EPOC CRC16 are not “settings.” Package name, UID3, vendor, capabilities, and cert/key already move with the project.

## How to read a cluster

| Class | Meaning |
|---|---|
| **1. Must stay hardcoded** | Protocol / format / ABI. Changing it produces invalid SIS, RSC, or E32. |
| **2. Already configurable** | Env, `symdev.toml` (`Manifest`), `.pkg` renderer inputs, or CLI. Pointed at a type/API. |
| **3. Should be project config** | Belongs on Manifest / `.pkg` / MMP / a future field. Frozen today from hello golden or scaffold defaults. |
| **4. Should not be user-config** | Toolchain internals, Wine adapter, recorded signatures, algorithm choices that must match MakeSIS. |

**False configurability** (called out in its own section): exposing a recorded word such as type-16 `0x21` as `sis.options = 0x21` without knowing what MakeSIS meant. The encoder already has constructors that take those words; that is not a product API.

---

## Cluster table

| Cluster | Class | Where it lives today |
|---|---|---|
| SIS / RSC / EXE UID1, SIS UID2, RSC UID2, field `KIND`s, 4-byte pad | 1 | `SisUid`, `RscUid`, `SisEncode::KIND`, `SisField::bytes` |
| EPOC CRC16, SIS checksums 34/35 | 1 | `UidCrc::checked`, `epoc_crc16`, `SisChecksum34::of` |
| zlib algorithm 1 + level 6 (controller); algorithm 0 (file bytes); SHA-1 file hash header | 1 | `SisCompressed::zlib`, `SisData32`, `SisHash` |
| TYPE=SA words `0x21` / `0x14` / `0x0d` / `0x1a`, products trailer `0x12`, type-40 `0` | 1 (until a pkg grammar exists) / **false knob** | `SisUnsigned::encode` → `unsigned_parts` |
| Platform UID `0x102752AE`, product name `S60ProductID`, version `0,0,0` | 1 for this product (S60 3rd FP2 / E52) | `render_pkg`, `unsigned_parts` |
| EXE `elf2e32 --uid1=0x1000007a`, `--fpu=softvfp`, `--targettype=EXE`, link `-Ttext 0x8000` | 1 | `GcceBuild` |
| Six user-grantable cap **set** (policy) | 1 (policy) + 2 (which of the six) | `USER_GRANTABLE`, `capability_word` |
| `symbian.uid3`, name, version, vendor, capabilities, cert/key | 2 | `Manifest`, `SisPackage` |
| Toolchain paths, Wine path, sign password | 2 | `Toolchain::from_env`, `SisTools::from_env`, `SYMDEV_SIGN_PASSWORD` |
| Install dest `!:\sys\bin\{name}.exe`, `&EN` / language id `1`, `TYPE=SA` token, localized vendor | 3 | `render_pkg` / `unsigned_parts`; MMP `TARGETPATH` parsed, unused |
| Signing DN `CN=Joe Bloggs…`, cert serial 1, `-expdays 3650` | 2 (DN: `signing.subject`, RFC 4514; default stays the recorded example) / 4 (serial, OID, key size) | `SelfSignedDsa::generate_for`, `SisPackage::subject` |
| Hello UID `0xe79e4cf9`, Vendor / Vendor-EN, stamp 2026-09-17 | tests only | golden constructors, not live `SisPackage` |
| Month 0-based in type 6; language id 1 | 1 (encoding) / 3 (which languages) | `SisDate`, `SisLanguage::new(1)` |
| DSA 1024/160 vs makekeys `-len 2048`; RFC 6979 `k`; `/usr/bin/wine` default; recorded tool argv | 4 | `SelfSignedDsa`, `SisTools`, `RcompTool`, `UidCrcTool` |
| `GcceBuild` compile/link argv, soname `{000a0000}`, no `-fPIC` | 4 | `GcceBuild::compile_args` / `link_args` |
| MMP `UID` / `CAPABILITY` / `LIBRARY` / `SYSTEMINCLUDE` | parsed, **not wired** (gap, not a new TOML table) | `parse_mmp` → `Mmp`; `GcceBuild::build` ignores them |

---

## 1. UIDs

### SIS file header — **must stay hardcoded** (UID1/UID2); package word is already config

`SisUid` (`crates/symdev-build/src/sis/uid.rs`):

```rust
pub const UID1: u32 = 0x1020_1a7a; // SIS file UID (experiment 14)
pub const UID2: u32 = 0;           // this SDK makesis; not a wiki default
pub fn new(package: u32) -> Self;  // UID3 = package UID
```

UID1 is the SIS container magic. UID2 is recorded `0` on this FP2 `makesis`. The fourth word is `UidCrc`, not an independent setting. Live encode already passes `SisUid::new(spec.uid3)` (`SisUnsigned::encode`).

### Platform UID `0x102752AE` — **must stay hardcoded** for this product

Verified: omitting it produces “App is incompatible with phone” ([uids-capabilities-signing.md](uids-capabilities-signing.md)). Locked: S60 3rd FP2 / Nokia E52. `render_pkg` and `unsigned_parts` both write `[0x102752AE], 0, 0, 0, {"S60ProductID"}` / `SisPkgUid::new(0x1027_52ae)` plus `SisVersion::new(0, 0, 0)`.

A later multi-device product could map `target.device` → platform UID. `Manifest.target.device` is already an enum with one value (`nokia-e52`); it does **not** currently change this word. Do not expose a free-form `platform_uid` hex while the device list is one phone.

### EXE UID1 `0x1000007a` — **must stay hardcoded** (KExecutableImageUid)

`GcceBuild::elf2e32_args` always passes `--uid1=0x1000007a` (Verified pipeline fragment). `--uid3` comes from `GcceBuild.uid3` (manifest). UID2 for EXE is **Unknown** / not passed.

### Test / self-sign UID3 — **already configurable**

`Manifest.symbian.uid3` (`symdev-manifest` `parse_uid3`): must be `0x` + 8 hex digits in `0xA0000000–0xAFFFFFFF` or `0xE0000000–0xEFFFFFFF`. Protected `< 0x80000000` is a hard error. CLI `build` / `package` require it (`uid3 required for build`).

Scaffold derives a test-range UID (`scaffold::uid3_for_name`, FNV-1a then `0xE0000000 | (h & 0x0FFFFFFF)`). That is **not** the blog hello `0xe79e4cf9`. `0xe79e4cf9` appears in goldens and `GcceBuild` unit fixtures only.

### RSC UID1 / UID2 — **must stay hardcoded**; UID3 is the app

`RscUid` (`crates/symdev-build/src/rcomp/mod.rs`):

```rust
pub const UID1: u32 = 0x101f_4a6b; // Unicode resource file
pub fn new(uid2: u32, uid3: u32) -> Self;
```

Experiment 41: both SDK `_reg.rsc` goldens use UID2 `0x101f8021` (registration resource). UID3 is the example app (`0xa00001f4`, `0xe80000a6`). When `_reg.rsc` is wired, UID3 should follow `Manifest.symbian.uid3`. UID1/UID2 are the resource-file ABI, not TOML.

MMP `UID` is parsed (`Mmp.uid`) and **not read** by `GcceBuild::build`. Traditional Carbide identity lived there; this product’s identity is the manifest. Wiring MMP UID without a mapping experiment would invent argv.

---

## 2. Capabilities and TYPE=SA words

### Which capabilities — **already configurable** (within the six)

`validate::USER_GRANTABLE`: `LocalServices`, `NetworkServices`, `ReadUserData`, `WriteUserData`, `UserEnvironment`, `Location`. Privileged names are a hard error. `GcceBuild::elf2e32_args` joins them with `+`; empty list **omits** `--capability=` (elf2e32_next rejects an empty argument).

SIS type 41 is **derived**, not a free hex:

`capability_word` in `sis/package.rs` maps those six names onto bits 13, 14, 15, 16, 17, 19. All six → `0x000be000` (hello type-41, experiment 29). Unknown names error: `SIS capability bits not yet derived from pkg`.

The **set of six** is policy (locked: self-sign only), not a knob to add `TCB`.

MMP `CAPABILITY` is parsed into `Mmp.capability` and unused. Driver uses the manifest list.

### TYPE=SA option words `0x21` / `0x14` / `0x0d` / `0x1a` — **must stay hardcoded** until pkg grammar exists; **false configurability** if exposed now

`unsigned_parts` (comment in tree):

```rust
SisWords16::new(SisWords::new(vec![0x21])), // ponytail: recorded TYPE=SA &EN one-file words
SisWords19::new(SisWords::new(vec![0x14])),
// inside SisFiles:
SisWords::new(vec![0x0d]),
SisWords::new(vec![0x1a]),
```

Plus `SisProducts::payload` appends a trailing type-2 word `0x12`, and the controller ends with `SisU32::new(0)` (type 40).

These are leftover MakeSIS controller words from experiment-7 `TYPE=SA` + `&EN` + one EXE. Specs explicitly refuse C names (`SisWords16` is not “Options”). Experiment 23: type-16 `0x21` is **not** the EN language id (`1`).

`render_pkg` also hardcodes the token `TYPE=SA` and `&EN`. A real `.pkg` author might choose `TYPE=SA` vs other SIS types, extra files, or more languages — that is a **pkg grammar** problem, not four hex knobs.

---

## 3. Vendor strings, DN, dates, zlib

### SIS vendor / names — **already configurable** (vendor); localized name is a gap

`Manifest.symbian.vendor` (default `"symdev"`). `SisPackage` passes it as both `vendor` and `vendor_localized`. Hello golden used `"Vendor"` / `"Vendor-EN"` (experiment 17). A localized vendor line `%{"…"}` vs `:"…"` is **should be project config** if we grow pkg languages; today one string is enough for SA+EN.

Package **name** and **version** are `Manifest.package`. They flow into `SisInfo` (`SisString` name array, `SisVersion`).

### Signing DN `CN=Joe Bloggs…` — **should be project config**

Copied from this SDK `makekeys` Example Usage (experiment 8), **not** from the SIS vendor.

- Wine argv still: `dname()` → `"CN=Joe Bloggs OU=Development O=Acme Ltd C=GB EM=noone@nowhere.com"` (`SisTools::makekeys_args`). CLI `package` no longer calls Wine `makekeys`.
- Native: `SelfSignedDsa::SUBJECT` = `"CN=Joe Bloggs,OU=Development,O=Acme Ltd,C=GB,emailAddress=noone@nowhere.com"`.

A real author would put their CN/O/C on the cert. Smallest surface: `signing.dname` (or reuse `symbian.vendor` for `O=` / `CN=`). Do not pretend the SIS vendor string is the cert subject — experiment 40 recorded they differ.

### Cert serial, OID, `-expdays 3650` — **should not be user-config** (except maybe expiry later)

`SelfSignedDsa`: serial `1`, OID `1.2.840.10040.4.3` (`dsaWithSHA1`), `EXPDAYS = 3650` (Verified 9.2+ tools). Serial/OID are X.509 layout matching MakeSIS-era certs. Expiry could later follow `makekeys -expdays`; it is not why hello would not install.

### SIS datetime — **already live**; month encoding is format

Live `SisPackage::package` stamps `datetime_utc(SystemTime::now())`. Golden tests pin experiment-21 `SisDate::new(2026, 8, 17)` + `SisTime::new(15, 18, 24)` so Wine `makesis` can be byte-matched.

Type-6 month byte is **0-based** (September → `8`, experiment 21). That encoding is format (**class 1**). Do not add `sis.month_style` to TOML. `civil_from_unix_days` follows Howard Hinnant (calendar month 1–12). Whether live stamps currently agree with MakeSIS’s 0-based byte is an encoder question, not a project setting.

`SisInfo::payload` appends two extra `00` bytes inside the unpadded length (experiment 22). Format padding, not config.

### zlib level 6 — **must stay hardcoded** (controller); **not a knob**

`SisCompressed::zlib` uses `flate2::Compression::new(6)` with crate feature `zlib` (system libz). Experiment 34: only level 6 + default strategy + memlevel 8/9 byte-matched MakeSIS (`78 9c`). Other levels change type-34 CRC. File payload uses algorithm `0` (raw E32, not zlib) in `unsigned_parts`.

Do not add `compression_level` to the manifest. Installers expect this SDK’s deflate.

---

## 4. Wine, SDK paths, tool argv

### Env — **already configurable**

| Variable | API |
|---|---|
| `SYMDEV_EPOCROOT`, `SYMDEV_GXX`, `SYMDEV_LD`, `SYMDEV_ELF2E32`, `SYMDEV_GCC_LIB`, `SYMDEV_GCC_TARGET_LIB` | `Toolchain::from_env` (all required, no defaults) |
| `SYMDEV_WINE` | `SisTools::from_env`; empty/unset → `/usr/bin/wine` |
| `SYMDEV_SIGN_PASSWORD` | CLI `package_project`; `validate_password` (≥ 4 chars) |
| `SYMDEV_EKA2L1`, `SYMDEV_ROM` | research/skip only ([eka2l1.md](eka2l1.md)); not CLI |

`Manifest.toolchain.sdk` must be `s60-3rd-fp2` if present; compiler enum `gcce-14` / `gcce-15` defaults to `Gcce14`. **The driver does not read that enum.** Actual `g++`/`ld` are the env paths (this host: GCC 12.1.0 + ld 2.29.1, experiment 5). See false configurability.

### Default `/usr/bin/wine` — **already configurable** (env); the default is host convention

`SisTools`, `UidCrcTool` tests, and `RcompTool` tests use `/usr/bin/wine` as the recorded experiment-4 loader. Override: `SYMDEV_WINE`. Native `package` does not spawn Wine.

### Tool argv — **should not be user-config**

Recorded, never invented: `GcceBuild::{compile,link,elf2e32}_args`, `SisTools::{makesis,makekeys}_args`, `RcompTool::args` (`-u -o… -s… -i…`), `UidCrcTool::args`. Flags such as `-nostdlib`, `--entry _E32Startup`, `rcomp -u` are ABI/tool usage. A user “compiler flags” array would violate “never invent argv” and the no-`-fPIC` lock.

`RcompTool` / `UidCrcTool` are recorded adapters; `symdev build` still does **not** spawn `rcomp`.

---

## 5. CRC / checksum algorithms

**Must stay hardcoded.**

`UidCrc::checked`: 12-byte LE uid1/uid2/uid3; EPOC CRC16 on odd bytes (high 16) and even bytes (low 16). Byte step in `epoc_crc16` (experiment 13; same as CRC-16/XMODEM on these inputs).

`SisChecksum34::of` / `SisChecksum35::of`: that CRC16 over the **padded** inner `SisField::bytes()` (type 3 → 34, type 30 → 35). Experiment 32: IEEE CRC32, Adler, other CRC16 inits did **not** match.

The rotate/`^` constants are the algorithm, not settings. File SHA-1 is live (`Sha1::digest(spec.exe)`); the type-25 prefix words `[1, 0x25, 0x14]` are the recorded hash-descriptor (`1` = SHA-1, `0x14` = 20-byte digest). Changing them without a second golden is inventing SIS.

---

## 6. File destinations

Live dest is `format!("!:\\sys\\bin\\{}.exe", spec.name)` (`unsigned_parts`). `render_pkg` uses the same path. Hello golden: `!:\sys\bin\hello.exe` (experiment 7/29).

**Class 3:** a real app may install `_reg.rsc` to `!:\private\10003a3f\import\apps\` (Verified template; not in Wave 0 hello `.pkg`) or other files. MMP `TARGETPATH` is already on `Mmp` and unused. Smallest future surface: dest from pkg/MMP, default `!:\sys\bin\{target}`.

The `!:` drive and `\sys\bin\` for the EXE are the SA install convention for this hello, not SIS magic — but inventing other dests without a pkg line would still be a grammar change.

File **sizes** in the type-24 tail (`[size, 0, size, 0, 0]`) are derived from `exe.len()`, not hardcoded 3588 except in tests.

---

## 7. DSA key size 1024/160 vs 2048

**Should not be user-config.**

Wine `makekeys_args` still passes `-len 2048` (experiment 8). Frozen `hello.cer` is DSA **p 2048 / q 160**. Native `SelfSignedDsa::generate` uses `KeySize::DSA_1024_160` because `dsa` 0.6 has no 2048/160 and `DSA_2048_256` was ~192s per debug keygen (comment on `SelfSignedDsa`).

Phones verify the signature; they do not require Wine’s modulus size. Exposing `signing.key_size = 2048` would promise a parameter the native path cannot honor. Keep native FIPS 1024/160; do not add a TOML key.

Live SISX uses RFC 6979 `k`. Frozen `hello.sisx` used SignSIS random `k` — **cannot** byte-match without recording the nonce (**class 4**, not a setting).

---

## 8. Recorded SIS field kinds, month, language

**Field `KIND` constants — must stay hardcoded.** They are the SIS TLV type tags (`SisEncode::KIND`): string `1`, array/words `2`, compressed `3`, version `4`, … controller `13`, info `14`, languages `15`, words16 `16`, products `17`, product `18`, words19 `19`, chain `22`, file `24`, hash `25`, data `30`/`31`/`32`, checksums `34`/`35`, signatures `36`–`39`, trailer `40`, caps word `41`, outer wrap `12`. `SisField::bytes` pads payload to 4 bytes. Changing a kind emits a different (invalid) SIS.

**Language id `1` — format id for English; list is class 3.** `SisLanguages` / `SisLanguage::new(1)` matches `.pkg` `&EN` (experiment 23). Do not invent other language IDs without a pkg `&XX` table. Multi-language names would be pkg grammar, then `SisArray` of ids + matching name strings.

**UTF-16-LE without BOM/NUL** (`SisString::payload`) is encoding, not config.

---

## 9. Other literals a project author would notice

| Literal | Class | Notes |
|---|---|---|
| `GcceBuild::linkas` `{000a0000}` | 4 until version mapping is specified | Experiment-5 soname; must match `--linkas`. Not `package.version`. |
| `-O2`, `-march=armv5t`, `-msoft-float`, `-D__S60_3X__`, HRH `symbian_os_v9.3.hrh` | 4 | Wave 0 recorded compile; no `-fPIC`/`-fPIE`. |
| `-Ttext 0x8000 -Tdata 0x400000` | 1 / 4 | Verified E32 load addresses. |
| Fixed `.dso` set (`euser`, `dfpaeabi`, …) | 4 now; MMP `LIBRARY` is the traditional extra list | `GcceBuild::link_args` ignores `Mmp.library`. Real apps will need MMP wiring, not TOML `libs = []`. |
| `signing.mode = "self-signed"` only | 1 (product) | `devcert` rejected. |
| Default vendor `"symdev"` | 2 (overridable) | Omitted `[symbian]` uses that default. |
| Scaffold `capabilities = []` | 2 | Author adds from the six. |
| `Device::NokiaE52` / `Language::Cpp` only | 1 for this phase | Extra devices/languages are new specs. |
| `SisInfo` extra `[0, 0]` | 1 | Experiment 22 leftover inside length 150. |

---

## Recommended config surface (smallest)

Do **not** add a parallel `symdev.toml` for SIS internals. Grow the types that already exist.

### Stay on `Manifest` (already)

| Field | Type | Feeds |
|---|---|---|
| `package.name` | `Package.name` | `.pkg`, soname, dest basename, cert filenames |
| `package.version` | `(u32,u32,u32)` | `SisVersion` / `.pkg` triple |
| `symbian.uid3` | `Option<u32>` | `GcceBuild.uid3`, `SisUid` / `SisPkgUid`, `--uid3` |
| `symbian.capabilities` | `Vec<String>` | elf2e32 `--capability=`, `capability_word` → type 41 |
| `symbian.vendor` | `String` | `.pkg` vendor lines, `SisInfo` vendor (+ localized until split) |
| `signing.cert` / `signing.key` | `Option<PathBuf>` | `SisPackage` existing pair; else `SelfSignedDsa::generate` |

### Stay env (host machine, not the project)

Toolchain binaries and SDK root. `SYMDEV_WINE` if anyone still calls `SisTools`. `SYMDEV_SIGN_PASSWORD` for encrypted Wine keys (native PKCS#8 keys accept empty). Never put SDK/ROM/password in git.

### Next smallest project fields (when an author actually needs them)

| Need | Where it belongs | Do not put it |
|---|---|---|
| Cert CN/O/C | `signing.dname` (or structured DN) on Manifest | SIS type-16 word |
| Extra install files / `_reg.rsc` dest | `.pkg` / MMP `TARGETPATH` + `START RESOURCE` | free hex dest UID |
| Extra link libs / includes | MMP `LIBRARY` / `SYSTEMINCLUDE` **wired into** `GcceBuild` | a second library list in TOML |
| Localized vendor / more `&XX` | pkg language grammar → `SisLanguages` + name arrays | `language_id = 1` as a magic integer |
| Platform UID | only if `target.device` grows; map enum → `0x102752AE` | user-typed `0x…` |

### Stay constant (not Manifest, not env)

SIS/RSC/EXE UID1, SIS UID2, RSC UID2 for `_reg.rsc`, all `KIND`s, pad rule, EPOC CRC16, zlib level 6 + algorithms 0/1, TYPE=SA words, platform UID + `S60ProductID` for E52, EXE uid1, FPU `softvfp`, six-cap policy, DSA-SHA1 OID, hash descriptor `[1, 0x25, 0x14]`, recorded g++/ld/elf2e32/rcomp argv, no `-fPIC`, DSA 1024/160 native keygen.

---

## False configurability

These would look like “settings” and would not help Yevhenii ship a second app:

1. **`sis.words16 = 0x21` (and `0x14` / `0x0d` / `0x1a` / `0x12`)** — recorded TYPE=SA &EN one-file leftovers. Specs forbid inventing C names. A different pkg (two files, another `TYPE=`) needs a **derived** encoder, not a hex dump. `SisWords16::new(SisWords::new(vec![0x21]))` is an implementation constructor, not a product API.

2. **`toolchain.compiler = "gcce-14"`** — already in TOML, **ignored** by `GcceBuild`. The compiler is `SYMDEV_GXX`. Changing the enum does not change argv.

3. **`compression_level = 6`** — MakeSIS compatibility, not size tuning. Wrong level → wrong type-34 CRC → not the SIS this SDK writes.

4. **`signing.key_bits = 2048`** — Wine makekeys flag vs native `DSA_1024_160`. A knob here lies.

5. **`elf2e32.uid1 = 0x1000007a`** — EXE ABI. Putting it in TOML invites a protected UID on a self-signed EXE.

6. **`rcomp.uid1 = 0x101f4a6b`** — resource-file magic.

7. **MMP `UID` as a second identity** — parsed, unused; manifest `uid3` is the product identity. Dual sources without a mapping rule.

8. **`target.device = "nokia-e52"` as if it selected platform UID** — it does not; `0x102752AE` is compiled in. The enum only rejects other device names.

9. **SignSIS `k` / frozen type-39 blob** — recorded so `hello.sisx` can be composed in tests (`SisController::with_signatures`). Live sign always generates a new `k`. Not a project file.

10. **Calendar “use 0-based months” in TOML** — the SIS byte is 0-based; the author should never see it.

---

## Why hello goldens still contain `0xe79e4cf9`

T2 required native `SisUnsigned::encode` to **byte-equal** experiment-7 `hello.sis` (experiment 38). That froze name `hello`, UID `0xe79e4cf9`, version `1,0,24`, vendor `Vendor` / `Vendor-EN`, dest `!:\sys\bin\hello.exe`, caps → `0x000be000`, stamp, and the TYPE=SA words. `encode_unsigned_sis_uses_project_fields_not_hello_goldens` already asserts a different name/UID/vendor does **not** emit that file.

Live `SisPackage` uses the manifest + `now()` + live SHA-1. The goldens are tests, not the default app identity. Scaffold writes a fresh `0xE…` UID and vendor `symdev`.

---

## Sources (in-tree)

- `crates/symdev-manifest/src/schema.rs` (`Manifest`, `Symbian`, `Signing`)
- `crates/symdev-manifest/src/validate.rs` (`USER_GRANTABLE`, `parse_uid3`, vendor default)
- `crates/symdev-build/src/sis/package.rs` (`SisUnsignedSpec`, `unsigned_parts`, `capability_word`, `SisPackage`, `dname`, `SisTools`)
- `crates/symdev-build/src/sis/makekeys.rs` (`SelfSignedDsa`)
- `crates/symdev-build/src/sis/{uid,field,words,files,compressed,checksum,datetime,language,products,info,controller,sign}.rs`
- `crates/symdev-build/src/{pkg,driver,toolchain,uidcrc}.rs`
- `crates/symdev-build/src/rcomp/mod.rs` (`RscUid`)
- `crates/symdev-cli/src/{main,scaffold,cli}.rs`
- [experiment-backlog.md](experiment-backlog.md) experiments 5–8, 13–14, 21, 23, 25–30, 32, 34, 38–41
