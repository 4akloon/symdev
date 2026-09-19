# EKA2L1 bring-up (M5 spike on this Linux host)

Investigation only. No implementation. Does **not** start T4, does **not** add `symdev emu`, does **not** download ROM/SDK/firmware, does **not** claim E52.

Facts are **Verified** against in-tree docs/types at `origin/main` `0d68415` unless labeled **Unknown** / **Needs experiment** / **Hypothesis**. Upstream EKA2L1 facts are cited; they are **not** observed on this host (experiment 10 remains `skip`). Do not invent EKA2L1 argv. Do not add env vars beyond `SYMDEV_EKA2L1` / `SYMDEV_ROM` until a run records them.

Cites: [eka2l1.md](eka2l1.md); [experiment-backlog.md](experiment-backlog.md) parked table + experiments 8–11; [legacy-sdk-leftover.md](legacy-sdk-leftover.md); [rust-sdk-idea.md](rust-sdk-idea.md); [uids-capabilities-signing.md](uids-capabilities-signing.md); [licensing.md](licensing.md); [m0-bare-metal-runbook.md](m0-bare-metal-runbook.md) ch.11–12; north-star spec §12 / §14.2 / §18; `EmulatorBackend` (`crates/symdev-core/src/traits.rs`); CLI `new`/`build`/`package`/`deploy` only.

Locked (not reopened): Linux host; never curl SDK/ROM; never commit ROM/`.sis`/`.sisx`/`.cer`/`.key`; EKA2L1 is GPL-3.0 **process only**; skip without blobs; **emulator success ≠ E52 supported**.

## Executive answer

**Now:** another agent owns **T4 E32 header**. After that, leftover **A** is full native `Elf2E32::encode` (still spawn `SYMDEV_ELF2E32` until encode actually works). **B** (rustc + libstd) later. **C** (replace EPOCROOT) no.

**Emulator until a real E52 exists:** unpark **experiment 10** on *this* Linux host as a human spike, not CI. Official Linux artifact is `EKA2L1-Linux-x86_64.AppImage` ([continuous release](https://github.com/EKA2L1/EKA2L1/releases/tag/continous)). **AppImage is on disk** at `/home/genius/Downloads/EKA2L1-Linux-x86_64.AppImage` (chmod +x; running the file *is* the install — not apt). **ROM/VPL/RPKG is not.** Without a phone dump, the emu cannot boot S60. This repo cannot fetch firmware. SIS install in the GUI is **File → Install → Package** ([install apps](https://eka2l1.github.io/quickstart/basic/installapp)). `--help` tokens are recorded below (addendum 2026-09-18 AppImage). Self-signed `0xE…` `hello.sisx` **might** install; that is **Needs experiment**, not a claim. Do not set `SYMDEV_ROM` to WINSCW/EPOCROOT.

**Sequence:** the spike can run **in parallel with T4**. The emulator consumes a **packaged SIS**, not native elf2e32. Wave 0 already produces `hello.exe` via Linux `elf2e32_next` and `hello.sisx` via native `symdev package` (and the frozen experiment-8 file). T4 is not a blocker for “run in emu.”

---

## 1. What we are doing now (A / B / C)

| Track | What | When |
|---|---|---|
| **T4 (in flight)** | Native E32 *header* encode in `symdev-elf2e32`. UID prefix already on `main` (`E32Uid`). This slice is the rest of the header. **Do not implement here.** | Other agent / `t4-e32-header` |
| **A leftover (after T4)** | Full `Elf2E32::encode` (ELF parse, imports, relocs, deflate, header CRC). Wave 0 **keeps spawning** Linux `SYMDEV_ELF2E32` (`elf2e32_next`) until encode byte-works. EPOCROOT headers / `.dso` / `--libpath` stay. | After T4 header lands |
| **B** | Custom rustc target + **libstd** on EKA2. North-star non-goal. **Do not implement.** | After C++ hello on a **stock** E52 (Hardware M0), not after emu |
| **C** | Reimplement S60/Avkon/E32 so EPOCROOT goes away. **Never for stock E52.** | No |

A is the current *product*. T4 is still A. B is a different compiler. C is a different OS. Mixing the three is how “Rust SDK” sounds cheap ([rust-sdk-idea.md](rust-sdk-idea.md)).

Parked on this host, unchanged by this note: **T5** macOS GCC; **M3** SSH; **E52** experiment 11. Do not flip those skips from a docs file.

---

## 2. Hello `.sisx` we can already produce on Linux

Wave 0 on this host does **not** need T4 or Wine PE for a copyable package.

| Artifact | How | Where |
|---|---|---|
| `hello.elf` | `arm-none-symbianelf-g++` 12.1.0 + GNU ld **2.29.1** (experiment 5) | `$HOME/src/symdev-experiment-5/` (outside git) |
| `hello.exe` | Linux `elf2e32_next` 3.0 Build 2 (experiment 6); `symdev build` spawns the same | same dir; `build/hello.exe` from CLI |
| Frozen `hello.sisx` | Wine `signsis` (experiment 8), 5172 bytes | `$HOME/src/symdev-experiment-5/hello.sisx` |
| Live `hello.sisx` | `symdev package` — native `SisUnsigned` + `SelfSignedDsa` (experiments 38–40). No Wine. | `build/hello.sisx` |
| Path print | `symdev deploy` prints that path; it does not copy to emu or phone | CLI only |

Scaffold UID3 is test-range `0xE…` (`uid3_for_name("hello")` → `0xef9f2cab`). Frozen goldens use blog UID `0xe79e4cf9`. Both are self-signable. Platform UID `0x102752AE` is in the pkg. Caps are the six user-grantable. Hello is `E32Main` + `e32cons` (`Console::NewL` / `Write`), **not** Avkon. The `.pkg` has **no** `_reg.rsc`. Whether launch needs that resource is still **Unknown** (experiment 9; waits on 10/11).

AppImage: `/home/genius/Downloads/EKA2L1-Linux-x86_64.AppImage`. Firmware (user-supplied, 2026-09-18): zip `~/Downloads/Nokia_E52_RM-469_v091.004.zip` extracted **outside git** to `/home/genius/sdk/e52-firmware/`. VPL: `.../Firmware/RM469_0591670_091.004_001.vpl` (RM-469, SW 091.004, EURO2). Set `SYMDEV_EKA2L1` and `SYMDEV_ROM` to those paths. Do not commit the zip/fpsx. Device install is **GUI only** (no CLI token). WINSCW/EPOCROOT are still not ROM.

---

## 3. EKA2L1 feasibility (unpark experiment 10 here, not CI)

[EKA2L1](https://github.com/EKA2L1/EKA2L1) is an experimental Symbian/N-Gage emulator: it **emulates** the kernel and **reimplements** critical app servers ([README](https://github.com/EKA2L1/EKA2L1)). GPL-3.0. Invoke as a **separate process**; never vendor source ([licensing.md](licensing.md); spec §12).

**M5 in the spec** is *headless install + launch + screenshot in CI* (north-star §18). That is **not** this spike. This spike is experiment 10: a human on this Ubuntu box installs a user-supplied binary + ROM, then tries our `.sisx`. Pass/fail stays in research notes, never `cargo test`. A later `EmulatorBackend` / optional `symdev emu` is post-§17; clap today is four verbs, and tests **forbid** listing `emulator`.

### 3a. Package name (Linux)

| Kind | Name | Status |
|---|---|---|
| Official CI artifact | `EKA2L1-Linux-x86_64.AppImage` | On disk: `/home/genius/Downloads/EKA2L1-Linux-x86_64.AppImage` (94 452 216 bytes, chmod +x 2026-09-18). GitHub tag `continous` listed the same name. |
| Ubuntu/Debian apt | none observed | Do not invent a distro package |
| Snap / Flatpak | not in upstream README | skip |
| From source | clone `--recurse-submodules`, CMake, Qt5/Qt6, build target `eka2l1_qt` ([BUILDING.md](https://github.com/EKA2L1/EKA2L1/blob/master/BUILDING.md)); Linux: GCC 10.1+ / Clang 10 (Clang not in their CI) | optional if AppImage fails on this distro |

README: **no official maintainer for Linux/OSX** — if the AppImage does not start, that is an upstream issue, not a symdev bug. User downloads or builds **themselves**. This repo never curls the AppImage.

Set `SYMDEV_EKA2L1` to the absolute path of that AppImage (or the `eka2l1_qt` binary). Do not invent extra env vars in [eka2l1.md](eka2l1.md).

### 3b. ROM / NAND — user-supplied; we cannot fetch

EKA2L1 does not ship a ROM. Device install is documented for PC including Linux ([install a device](https://eka2l1.github.io/quickstart/basic/installdevice)):

| Method | Who it is for | What the user must place on disk |
|---|---|---|
| **Device dump** (ROM + RPKG) | All versions; required for S60v1/v2 | ROM dump + RPKG (Z: drive archive). Dump path: ROMPatcher+ “Dump ROM” + [Dumber](https://github.com/EKA2L1/Dumber) “Dump RPKG” ([wiki](https://github.com/EKA2L1/EKA2L1/wiki/Dumping-the-ROM-and-ROFS)). Dumber says jailbreak first. |
| **Firmware (VPL)** | S60v3 and up (E52 is S60 3rd FP2) | A firmware tree **with a VPL**. Upstream: these are “distributed widely.” **This repo still cannot fetch them.** Same rule as ROM: user-supplied, outside git, legal access only. |

Wiki alternative layout (not observed here): `data/roms/RM-###/SYM.ROM` plus extracted Z: under `data/drives/z/RM-###`. Treat as **Unknown until experiment 10** copies what this AppImage actually writes. Do not add `SYMDEV_RPKG` / `SYMDEV_VPL` until that run. Extra files sit next to the ROM path the user already sets as `SYMDEV_ROM`.

Nokia E52 is **RM-469**. Upstream’s “most maintained” S60v3 example for N-Gage 2.0 is **Nokia 5320**, not E52. Experimental S60v3 FP1 named in 0.0.8.1 notes is **N81**. A 5320/N81 dump is a *different phone*. Using it can de-risk SIS install; it does **not** authorize an E52 claim.

### 3c. SIS install path

**GUI (documented):** install a device first, then **File → Install → Package** and pick the `.sis` / `.sisx` ([install apps](https://eka2l1.github.io/quickstart/basic/installapp)). App list updates after SIS install (0.0.8.1 notes).

**CLI:** `--help` observed on this host 2026-09-18 (verbatim in addendum). Recorded tokens include `--install,-i : Install a SIS.` and `--app,-a,--run` (name, UID, or virtual EXE path). `--version` is **not** a flag (`Unknown argument: --version`); version string is the log line `EKA2L1 v0.0.1 (master-e169852a5)`. Device install is **not** in `--help` — GUI **File → Install → Device** still required. Do not invent extra argv. `--install` was **not** exercised (no ROM, experiment 10 still `skip`).

Headless screenshot automation is **M5**, after a GUI/CLI path is recorded.

### 3d. Self-signed `0xE` UID `hello.sisx`

In-tree (Verified): test UID `0xE0000000–0xEFFFFFFF`; self-sign only; six user-grantable caps; platform `0x102752AE`. On a **stock** E52 the verified App. Mgr settings are Software installation = All, Online certificate check = Off. That is hardware, not emu.

On EKA2L1: **Needs experiment.** Upstream documents SIS/SISX install; it does not document certificate policy, “unsigned” vs self-signed DSA, or UID range checks. **Hypothesis** (not a pass): a user-grantable self-signed `0xE` package is the intended developer path and is more likely to install than a protected UID / TCB package. Failure modes to record, not guess: cert rejected, platform UID mismatch vs the *dumped* device, missing `_reg.rsc` so it installs but does not appear, console server missing so it appears then dies.

Do not claim “self-signed hello installs in EKA2L1” until experiment 10 says `pass` with a screenshot or log.

### 3e. Graphics / console

Hello is a **fullscreen Symbian console** (`e32cons`), not a host terminal and not Avkon. EKA2L1 is a **Qt GUI** emulator (SDL2 for pads). If `Console::NewL` / `Write` / `Getch` are implemented in the reimplemented servers, the window should show `Hello, world!` and wait for a key. That is **Needs experiment**.

Without `_reg.rsc`, S60 may not list the app. EKA2L1’s own app list after SIS install may still show it — or may not. Try-without = current hello `.sisx`. Try-with waits on a hello `_reg.rss` (T3 remainder; not this spike).

### 3f. Known gaps vs stock E52 (do not flatten)

- **Reimplementation, not the phone.** Kernel emulated; app servers rewritten. Compatibility is “a limited subset of Symbian applications” ([README](https://github.com/EKA2L1/EKA2L1)). Games/N-Gage are the project’s centre of gravity, not Eseries console EXEs.
- **Device identity.** E52 (RM-469, S60 3rd FP2, 240×320, QWERTY-less candybar) is not named in the install guide or in a compatibility-list hit for “E52”. A 5320 dump is not an E52.
- **Linux is second-class.** No official Linux maintainer; AppImage may lag.
- **No radio / BT / Nokia Suite / hardware keys.** Interim de-risk of *the SIS*, not of device bring-up.
- **Dumping needs a jailbroken phone + ROMPatcher+ / Dumber** if using Device dump. Firmware+VPL is the other legal-access path the user already has — still not something this repo fetches.
- **Experiment 10 pass ≠ Hardware M0.** Spec: emulator success does not mean “E52 supported.” Experiment 11 stays `skip` until a stock E52 is on the desk.

---

## 4. Ordered checklist (experiment 10)

Skip any step whose input is missing; record `skip`, do not curl.

1. Confirm frozen or live `hello.sisx` exists (experiment 8 path or `symdev package`). Do not commit it.
2. User installs EKA2L1 themselves (AppImage from [continous](https://github.com/EKA2L1/EKA2L1/releases/tag/continous), or a local CMake `eka2l1_qt`). **Done on this host:** AppImage at `/home/genius/Downloads/EKA2L1-Linux-x86_64.AppImage` (`chmod +x`). Suggest `export SYMDEV_EKA2L1=/home/genius/Downloads/EKA2L1-Linux-x86_64.AppImage` (not written to shell rc).
3. User places a ROM they may use (E52 dump preferred; any S60v3 dump is a weaker interim). Set `SYMDEV_ROM`. Place RPKG or VPL beside it if the installer asks. Never commit. **Blocked:** no `.vpl` / `.rpkg` / ROM `.img` on disk; no E52 dump in the SDK. Without a phone, wait, or point `SYMDEV_ROM` at a **legally possessed** S60v3 firmware+VPL they already have.
4. Run `"$SYMDEV_EKA2L1" --help`. **Done** — verbatim excerpt in the AppImage addendum. Those tokens are the only allowed CLI. Process does not exit after help (timeout 124); FUSE worked (no `--appimage-extract-and-run`).
5. GUI: File → Install → Device (Device dump or Firmware VPL). Record which method worked and the on-disk layout under the emulator’s data dir.
6. GUI: File → Install → Package → `hello.sisx`. Record dialog text (success, cert, incompatible, missing dependency).
7. Launch: from the emulator app list if present; otherwise only via a flag that `--help` actually listed. Record whether `Hello, world!` appears and whether a key dismisses it.
8. If install works but nothing launches: note `_reg.rsc` as the next hypothesis. Do not invent a hello `.rss` in this spike.
9. Write outcome `pass` / `fail` / `skip` in [experiment-backlog.md](experiment-backlog.md) experiment 10. Do **not** change experiment 11. Do **not** say E52.
10. Only after a recorded CLI exists: consider (later) wiring `EmulatorBackend` or a `symdev emu` verb. Not this spike. Not CI (M5).

---

## 5. In-repo vs on disk

| In this repo (docs / later code) | User must place (outside git) |
|---|---|
| This note; existing [eka2l1.md](eka2l1.md) skip rules | `SYMDEV_EKA2L1` → `/home/genius/Downloads/EKA2L1-Linux-x86_64.AppImage` (or a local `eka2l1_qt`) |
| Experiment 10 evidence after a run | `SYMDEV_ROM` → ROM and, if asked, RPKG or VPL |
| Optional later: `EmulatorBackend` impl + `symdev emu` (not authorized now; north-star forbade the verb in §17) | `hello.sisx` already at `$HOME/src/symdev-experiment-5/hello.sisx` or `build/hello.sisx` |
| Never: EKA2L1 sources, ROM, firmware, `.cer`/`.key` | Cert/key for live sign stay local if regenerating SISX |

Docker still has **no** EKA2L1. Default `cargo test` still does not spawn it.

---

## 6. Sequence vs T4

T4 reverse-engineers the **E32 file format**. Experiment 10 installs the **SIS wrapper**. They do not share a lock.

```
already: g++ → ld 2.29.1 → elf2e32_next → native package → hello.sisx
parallel: T4 header → later A full encode (spawn until green)
parallel: experiment 10 when user drops AppImage + ROM
later: B after Hardware M0; never C for this phone
later: experiment 11 when a stock E52 exists (only that unblocks “E52”)
```

Honest constraint: emu tests **packaging + a reimplemented SWInstall**, not “the E32 header we will one day encode in Rust.” A T4 bug would not show up in emu until Wave 0 stops spawning `elf2e32_next`.

---

## 7. What not to say

- Do not say “E52 supported,” “emulator supported,” or “M5 done.”
- Do not say T4 blocks running in emu.
- Do not say self-signed `0xE` hello **does** install in EKA2L1 (untested).
- Do not invent argv beyond the `--help` excerpt in the AppImage addendum (`--install` / `--app` exist; `--version` does not).
- Do not unpark experiment 10’s *outcome* from this note (still `skip` until blobs exist).
- Do not download ROM/firmware/SDK. Do not commit them.
- Do not add `symdev emu` while clap is locked to four commands.
- Do not start B or C.

---

## Sources

In-tree: [eka2l1.md](eka2l1.md); [experiment-backlog.md](experiment-backlog.md); [legacy-sdk-leftover.md](legacy-sdk-leftover.md); [rust-sdk-idea.md](rust-sdk-idea.md); north-star `docs/superpowers/specs/2026-09-16-symdev-m0-north-star-design.md` §12, §14.2, §18.

Upstream (not observed on this host):

- [EKA2L1/EKA2L1](https://github.com/EKA2L1/EKA2L1) README — kernel emulate / servers reimplement; Linux artifacts via Actions; no official Linux maintainer; limited app compatibility
- [continuous release](https://github.com/EKA2L1/EKA2L1/releases/tag/continous) — `EKA2L1-Linux-x86_64.AppImage`
- [Install a device](https://eka2l1.github.io/quickstart/basic/installdevice) — ROM+RPKG vs firmware VPL; PC Linux GUI; `--help`
- [Install apps](https://eka2l1.github.io/quickstart/basic/installapp) — File/Install/Package
- [Dumping the ROM and ROFS](https://github.com/EKA2L1/EKA2L1/wiki/Dumping-the-ROM-and-ROFS) — ROMPatcher+, Dumber RPKG
- [Dumber](https://github.com/EKA2L1/Dumber) — jailbreak then Dump RPKG
- [BUILDING.md](https://github.com/EKA2L1/EKA2L1/blob/master/BUILDING.md) — Qt UI, `eka2l1_qt`, GCC 10.1+
- [Releases notes 0.0.8.1](https://github.com/EKA2L1/EKA2L1/releases) — Linux AppImage; experimental S60v3 FP1 N81; SIS install refreshes app list

---

## Addendum: IA SDK dump ≠ ROM (2026-09-18)

Searched this host for the Archive.org listing `nokia_sdks_n_dev_tools` vs what EKA2L1 needs. Did **not** download torrents or ROM. Did **not** commit archives.

**Verdict: none of this listing is an E52 / EKA2L1 ROM.** SDK ≠ phone dump. Experiment 10 stays `skip`.

### What exists on disk

| Path | Size | What it is |
|---|---|---|
| `/home/genius/Downloads/nokia_sdks_n_dev_tools2/` | 13 GiB (88 files) | IA **Part 2** (`nokia_sdks_n_dev_tools2`). Part 1 (`nokia_sdks_n_dev_tools`) is **not** present. Torrent `nokia_sdks_n_dev_tools2_archive.torrent` (88 KiB) left untouched. |
| `/home/genius/Downloads/S60_3rd_Edition_SDK_Feature_Pack_2_v1_1_en.zip` | 477091187 | Wave 0 S60 3rd FP2 C++ SDK installer zip (InstallShield; 15 members, no `.img`/`.rpkg`/`.vpl`). |
| `/home/genius/sdk/S60_3rd_FP2/` | 467 MiB | Unpacked EPOCROOT: headers, 1140 `.dso`, `armv5` + **WINSCW**. WINSCW is the Windows SDK emulator, **not** EKA2L1. Extra FP2 zips would be redundant. |
| `/home/genius/sdk/S60_3rd_Edition_FP2_v1.1_installer/` | 467 MiB | Same installer unpacked (CABs). |
| `/home/genius/sdk/S60_3rd_FP2_examples/` | 164 MiB | SDK examples. |

Named Wave 0 zips **not** on disk (and not needed while EPOCROOT is unpacked): `S60_3rd_FP2_SDK_v1_1_1.zip`, `S60_SDK_3.2_v1.1.1_en.zip`. No `/home/genius/downloads`. Env still unset. AppImage later appeared (next addendum). Still **no** `*.rpkg` / `*.vpl` / ROM `.img` under Downloads, sdk, src, or common emu paths (rechecked 2026-09-18 after the AppImage).

### Classify (listing vs disk)

**Useless for EKA2L1** (wrong OS generation or Windows IDE; observed in Part 2): Java ME (SEMC CLDC, Java_ME_Developers_Library, Series 40 MIDP SDKs, nptsdk); Series 40 (`S40_*`); UIQ 2.1 (`100363-resource11.zip`); S60 2nd (`s60_2nd_fp2_sdk_msb.zip`, `S60_SDK_2_1_NET.zip`); Carbide.ui / Carbide.c++ `.exe`; Qt SDK `QtSdk-offline-win-x86-v1.2.1.exe` (1.7 GiB); Belle / Anna / Symbian^3 **theme** plugins (`Belle FP*.zip`, `E6 *.zip`, `S60v5.zip`, `Symbian^3 (Compiled).zip`); N97 theme plugin (not a 5th-ed C++ SDK); 2004 `NokiaCD*.iso` (docs + old Nokia SDKs); `Ngage Arena.iso` (PDF); `source.zip` (6.3 GiB Symbian FCL `.hg` bundles); `documentation.zip` (HTML `pdk/` + `sdk/` chunks, not a device PDK). Names like `S60v3 FP2.zip` / `E71.zip` / `E72.zip` are 3.6–4.7 MiB Carbide theme plugins (`com.nokia.tools.theme…`), not firmware.

Part 1 of that IA collection (UIQ / S60 1st–2nd / CodeWarrior / Perl / PyS60 / Qt Belle / N97 5th / PDK Belle, plus any extra FP2 C++ zips) is **not on this host**. Same classification if it appears later: IDEs and other OS generations do not boot EKA2L1; another FP2 C++ zip is headers/.dso for g++, still not a ROM.

**Might help only if they contained ROM/RPKG/VPL:** they do not. Quick listings (no extract of the 6.3 GiB tree): Wave 0 SDK zip, Part 2 theme/SDK/UIQ/2nd-ed zips, nested `documentation/{pdk,sdk}.zip`, `source.zip`, and the four ISOs have **zero** `.rpkg` / `.vpl` / `.img` / ROFS / `eka2l1` members.

**Missing for emu (not in this IA folder):** a user-entitled device dump (ROM+RPKG) or S60v3+ firmware tree **with a VPL**. Nokia E52 is RM-469. AppImage is now on disk (next addendum).

### What to put where

| Env | User places (outside git; this repo never fetches) |
|---|---|
| `SYMDEV_EKA2L1` | `/home/genius/Downloads/EKA2L1-Linux-x86_64.AppImage` (or a local `eka2l1_qt`). |
| `SYMDEV_ROM` | Absolute path to that ROM (RPKG or VPL beside it if the installer asks). **Unset — no VPL/RPKG/ROM `.img` found on disk.** |

Then resume [§4](#4-ordered-checklist-experiment-10). Do not treat WINSCW, EPOCROOT, or the IA dump as `SYMDEV_ROM`.

---

## Addendum: AppImage on this host (2026-09-18)

User-supplied binary. Did **not** curl it. Did **not** commit it. Did **not** fetch ROM. Did **not** write shell rc or git-config. Did **not** touch the `t4-e32-header` worktree.

### Path and how to run

AppImages are not apt-installed. Running the file **is** the install.

```
chmod +x /home/genius/Downloads/EKA2L1-Linux-x86_64.AppImage
export SYMDEV_EKA2L1=/home/genius/Downloads/EKA2L1-Linux-x86_64.AppImage
"$SYMDEV_EKA2L1" --help
"$SYMDEV_EKA2L1"
```

FUSE worked: `--help` printed without `--appimage-extract-and-run` or extra `libfuse2`. If a later run fails to mount, try `"$SYMDEV_EKA2L1" --appimage-extract-and-run --help` or install `libfuse2`. Process does not exit after `--help` (observed `timeout` 124); Ctrl+C / kill is needed. Qt log: `Could not find the Qt platform plugin "wayland"` — it still printed help.

`--help` also logs `Devices file not found` / `No current device is available. Stage two initialisation abort` — expected with no ROM.

### `--help` excerpt (verbatim)

```
I /home/runner/work/EKA2L1/EKA2L1/src/emu/qt/src/state.cpp:81 [Frontend.Cmdline]: EKA2L1 v0.0.1 (master-e169852a5)
	--help,-h : Display helps menu
	--listapp : List all installed applications
	--listdevices : List all installed devices
	--app,-a,--run : Run an app with given name or UID, or the absolute virtual path to executable.
			  See list of apps with --listapp.
			  Extra command line arguments can be passed to the application.

			  Some example:
			    eka2l1 --run C:\sys\bin\BitmapTest.exe "--hi --arg 5"
			    eka2l1 --run Bounce
			    eka2l1 --run 0x200412ED

	--device,-dvc : Set a device to be ran, through the given firmware code. This device will also be saved in the configuration as the current device.
			 Example: --device RH-29
	--install,-i : Install a SIS.
	--remove,-r : Remove an package.
	--fullscreen,-f : Display the emulator in fullscreen.
	--mount,-m : Load a folder/zip as a Game Card ROM.
	--keybindprofile,-kbp : Set a keybind profile to associate with the emulator launch. Don't include any file extension here.
	 Example: eka2l1 --kbp controller_for_octopus
	--mmcid,--cid,-cid : Set the MMC-ID for the mounted card
	--runng,--appng,-rng,-ang : Run a single N-Gage game inside the E drive
```

`--version` is **not** a token: stdout `Unknown argument: --version`. Version is only the log line `EKA2L1 v0.0.1 (master-e169852a5)`. `--install` was not run. Device install is not in this list.

### ROM next step (2026-09-18)

User placed `Nokia_E52_RM-469_v091.004.zip`. Extracted to `/home/genius/sdk/e52-firmware/` (not in git). Contains VPL + `prd.core.fpsx` / `rofs2` / `rofs3` / `uda` / APE fpsx — the Firmware (VPL) method for S60v3.

```
export SYMDEV_EKA2L1=/home/genius/Downloads/EKA2L1-Linux-x86_64.AppImage
export SYMDEV_ROM=/home/genius/sdk/e52-firmware/Nokia_E52_RM-469_v091.004/Firmware/RM469_0591670_091.004_001.vpl
"$SYMDEV_EKA2L1"
```

In the GUI: **File → Install → Device** → method **Firmware (VPL)** → Browse to `$SYMDEV_ROM` → pick variant if asked → Install. Then experiment 10: **File → Install → Package** `hello.sisx`. Emu with RM-469 firmware is still **not** stock E52 hardware (experiment 11 stays skip). Some VPL optional files (simlock, memory-card fpsx) are missing from the zip; install may still work.

---

## Addendum: Linux GL SEGV during econs / AknIconSrv (2026-09-18)

Host is **AMD `amdgpu`**, not NVIDIA. AppImage does **not** bundle `libGL`; it loads system Mesa (`libGLX_mesa.so.0`, unified `libgallium-26.0.8-*.so`). `enable-hw-gles1: false` already in `~/.local/share/EKA2L1/config.yml` (no vulkan/gles2 keys present; none invented).

| Run | Env that mattered | GL log | Outcome |
|---|---|---|---|
| 17:14:49–17:15:45 | `LIBGL_ALWAYS_SOFTWARE=1` (GL 4.5 still) | `Created a GLX context with version 4.5`; features `ETC2Dec;AnisotrophyFiltering;ES3.1_Compability` | SEGV after `on_cancel_button_clicked` then `econs` / fonts / `AknIconSrv` |
| 17:17:03–17:19:31 | same + `GALLIUM_DRIVER=llvmpipe` **and** `MESA_GL_VERSION_OVERRIDE=2.1` | **no** “Created a GLX context…” line; `glGetIntegerv` 1280; **missing `GL_ARB_draw_elements_base_vertex`**; empty supported-features | SEGV ~2s after econs (`Loaded library: akninit.dll` then `status=11/SEGV`) |
| 17:22:50– | llvmpipe **without** version override (below) | `Created a GLX context with version 4.5`; full features; **no** missing-extension list | GUI idle 90s+ (no `/dev/dri` fds = software). At 17:25:02 **Font Magnifier** launched: passed `AknIconSrv` / `akninit.dll` / draw stubs / MBM load. Guest thread then `Access violation` (not host SEGV). Host still **active** 17:27:15. |

`MESA_GL_VERSION_OVERRIDE=2.1` is the wrong hammer: EKA2L1’s OGL backend needs `GL_ARB_draw_elements_base_vertex` (GL 3.2+). llvmpipe already reports GL 4.5; forcing 2.1 hides that extension and still dumps.

Working launch env (no `--run`; `systemd-run --user --unit=eka2l1 --property=KillMode=process`):

```
DISPLAY=:0 QT_QPA_PLATFORM=xcb QT_OPENGL=software
LIBGL_ALWAYS_SOFTWARE=1 GALLIUM_DRIVER=llvmpipe MESA_LOADER_DRIVER_OVERRIDE=llvmpipe
__GLX_VENDOR_LIBRARY_NAME=mesa
```

Do **not** set `MESA_GL_VERSION_OVERRIDE`. Same `Exec=env …` on `~/.local/share/applications/eka2l1.desktop` and `~/Desktop/eka2l1.desktop`.

**Verified 2026-09-18 17:55–18:00 (this host, agent-driven GUI, no `--run`):** double-click **Dictionary** (UID `0x200159D0`) with the llvmpipe env above. Guest reached `AknIconSrv` / `akninit.dll` / Dictionary chrome. Host Qt stayed **active** >2 min, no ABRT/SEGV. Stock ROM process launch does **not** kill the host. Avoid Font Magnifier / Maps; they are optional and previously guest-AV.

---

## Addendum: hello.exe summons then host heap death (2026-09-18)

`hello.exe` is at `~/.local/share/EKA2L1/data/drives/e/sys/bin/hello.exe` (no `_reg.rsc`; not in the app list). Launch path that was actually used: **Emulation → Launch process** → `E:\sys\bin\hello.exe` (not File→Launch; that menu item is under Emulation). Not cold `--run`.

| Time | What | Host |
|---|---|---|
| 18:00:55 | `Loaded library: econs` then window-group opcode `0x30` (`EWsWinOpCancelTextCursor`) ×3, graphics opcode 403 (out of range), `free(): invalid next size` | ABRT status=6 |
| 18:06:19 | Same sequence with `integer-scaling: false` | ABRT status=6 |
| 18:08:09 | Same sequence with `cpu: dyncom` | SEGV status=11 |

`0x30` is already stubbed in upstream `wingroup.cpp` default (`ctx.complete(error_none)`). Config/env does not change that. `integer-scaling` and `cpu` were reverted to `true` / `dynarmic`.

**Cause (source, not guessed env):** `drivers::command_list::retrieve_next()` writes `base_[size_++]` with **no** `size_ < max_cap_` check. Cap is `MAX_CAP_COMMAND_COUNT` (12800). GLES callers flush via `need_flush()`; **window-server / econs do not**. Console redraw overflows the command array → heap smash → the `0x30` log is a symptom sitting next to the free.

**Patch (not built on this host):** `/home/genius/src/EKA2L1-econs-heap.patch` applied in `/home/genius/src/EKA2L1` (outside git). Grows the command list when full; also guards `set_text_cursor` `cmd_len` and names `CancelTextCursor` / `SetTextCursorClipped`. Rebuild needs CMake + Qt `eka2l1_qt`. This machine has **no** `cmake`, **no** Qt -dev, **no** passwordless sudo, **no** `pip`/`ensurepip`. Do not start the 2h build until those packages exist.

Idle AppImage relaunched 18:08:51 with the llvmpipe env; guest **not** booted in that instance. `enable-hw-gles1: false` unchanged.
