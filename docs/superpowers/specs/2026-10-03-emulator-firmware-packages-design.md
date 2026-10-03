# `emulator` and `firmware` packages — design

Status: approved in conversation with the owner on 2026-10-03. Required before symdev 0.4.0:
`cargo run` and `cargo test` (`2026-10-03-cargo-build-run-design.md`) need an EKA2L1 with
`--control` and `--data-dir`, and a firmware, and a clean machine must get both from
`install.sh` → `symdev new` → `cargo run` with nothing installed by hand. This is the
"emulator and firmware packages" item that the toolchain manager spec
(`2026-10-02-toolchain-manager-design.md`, §1 and §11) left to phase 2.

## 1. Decisions taken with the owner

| Question | Decision |
|---|---|
| Scope before 0.4.0 | Both: `emulator` (public) and `firmware;rm-469` (private) |
| How the EKA2L1 binary is built | By the fork's own CI (upstream's workflow, AppImage) from our integration branch; the recipe takes that artifact |
| C++ `symdev run` / `symdev test` | Same device choice and runner as Rust (one run path) |

## 2. The integration branch

- `symdev` in `4akloon/EKA2L1` is rebuilt on the current upstream `master` with every open PR
  of ours: #724, #726, #727, #728, #766, #767, #768, #769, #770, #771, #772. Conflicts are
  resolved on the branch; each PR branch stays as it is.
- Pushing it runs the fork's `.github/workflows/build.yml` (upstream's), whose Linux job builds
  `eka2l1-qt-x64.AppImage` with linuxdeploy and its Qt plugin.
- The branch is rebuilt whenever one of our PRs changes or upstream moves in a way we need; each
  rebuild that symdev adopts becomes a new `emulator` version.

## 3. `emulator;<yyyy.mm.dd>` (public bucket)

- Recipe `recipes/emulator/<version>/` in `symdev-packages`. It names the fork commit, the CI
  run and the artifact's SHA-256; it refuses an artifact whose hash differs (artifacts expire
  after 90 days, the package in R2 does not).
- Contents, from the AppImage extracted with `--appimage-extract` (no FUSE at run time):
  the extracted tree with its `AppRun` entry point (symdev starts the program the way the
  observed layout needs; that is recorded in experiment 115),
  the bundled Qt and other libraries, and `share/doc/eka2l1/` with EKA2L1's GPL-3.0 text and
  the notices of every bundled library (Qt under LGPL-3.0 and the rest, listed from the
  extracted tree).
- Corresponding source, published with the package through the publisher's existing source
  archive: the fork commit as a `git archive` including submodules, plus the source of the Qt
  version linuxdeploy bundled. Licence field: the SPDX expression the collected notices establish
  (at least `GPL-3.0-or-later` for EKA2L1 and `LGPL-3.0-only` for Qt).
- `host = "x86_64-linux"`. The AppImage's glibc floor is recorded in the recipe; if it is newer
  than the hosts we support, that is a finding for the owner, not a silent rebuild.

## 4. `firmware;rm-469;1` (private bucket only)

- Contents, in EKA2L1's data layout: `roms/rm-469/` (51 MB), `drives/z/rm-469/` (208 MB) and
  the device's entry from `devices.yml` (`RM-469`: `platver: epoc93fp2`, `firmcode: RM-469`,
  `model: N00`, …) as `device.yml`.
- Made on the owner's machine by `recipes/firmware/rm-469/1/`, whose script reads an EKA2L1
  data directory given by an environment variable and writes the archive; published with
  `publish private`. Firmware never enters git, CI or the public bucket (CLAUDE.md,
  `docs/research/licensing.md`).
- `host = "any"`.

## 5. symdev's side

- **Emulator resolution:** `SYMDEV_EKA2L1` if set; otherwise the installed `emulator` package
  of the version `Pins` names for this symdev release, installed automatically on first need
  (`cargo run`, `cargo test`, `symdev run`, `symdev test`, `symdev emulator start`), as `symdev
  build` installs the SDK. Host-specific settings (this host's software-GL variables) stay in
  the user's own wrapper reached through `SYMDEV_EKA2L1`; the package imposes none.
- **Firmware resolution:** `SYMDEV_EKA2L1_DATA` if set (today's behaviour: profiles from the
  user's EKA2L1 data); otherwise the `firmware;rm-469` package, installed automatically when a
  configured source offers it; otherwise an error naming both ways (configure the private
  source, or set `SYMDEV_EKA2L1_DATA`).
- **Profiles from the package:** a profile has its own drives C:, D:, E:, a generated
  `devices.yml`, and ROM and Z: taken from the installed package without copying. Whether
  EKA2L1 writes into Z: or the ROM is observed first, with the package files read-only; if it
  writes, the profile gets copies instead.

## 6. Verification

- **Unit:** the two resolution orders; a profile made from a package-shaped fixture tree;
  the recipe's artifact-hash refusal.
- **Real (experiment 115):** the extracted AppImage starts with `--data-dir` and `--control`,
  answers `apps.list`, installs and launches `hello`; whether Z:/ROM are written.
- **Acceptance:** an empty HOME with the private source configured, `~/.local/share/EKA2L1`
  hidden by `bwrap --tmpfs`: `install.sh` → `symdev new --lang rust` → `cargo run` installs
  `emulator` (public) and `firmware;rm-469` (private) by itself and shows the app (PID-bound
  screenshot); `cargo test` passes.
- `docs/research/licensing.md` gains the emulator and firmware rules.

## 7. Out of scope

Running the emulator in CI (needs a virtual display); firmware other than RM-469; macOS and
Windows emulator packages; `symdev device create` beyond profiles made on first need.
