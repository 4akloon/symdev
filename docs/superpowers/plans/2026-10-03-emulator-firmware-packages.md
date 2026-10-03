# `emulator` and `firmware;rm-469` packages — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** On a clean machine, `install.sh` → `symdev new --lang rust` → `cargo run` installs
an EKA2L1 with `--control` and `--data-dir` (`emulator;<V>`, public bucket) and the E52
firmware (`firmware;rm-469;1`, private bucket) by itself, and shows the app.

**Architecture:** The fork's CI builds the AppImage from our rebuilt integration branch. A
recipe in `symdev-packages` takes that artifact by its SHA-256, extracts it and adds the
notices. The corresponding source goes out through the publisher's existing `--source-code`
archive. The owner stages the firmware from his EKA2L1 data and publishes it privately.
symdev resolves `SYMDEV_EKA2L1` / `SYMDEV_EKA2L1_DATA` first and the pinned packages
second. It starts `<package>/usr/bin/eka2l1_qt` directly, and makes an emulator profile
whose ROM and drive Z are links into the installed firmware package.

**Tech Stack:** Rust 1.98.1 (edition 2024) in `symdev` and in `symdev-packages`
(`publish`, `pkgtools`); bash recipe drivers; GitHub Actions (the fork's `build.yml`,
`symdev-packages`' workflows); Docker `ubuntu:24.04` for the CI rehearsal; EKA2L1 (GPL-3.0,
always a separate process).

**Spec:** `docs/superpowers/specs/2026-10-03-emulator-firmware-packages-design.md`, which
extends `docs/superpowers/specs/2026-10-03-cargo-build-run-design.md` §5–§6 and the toolchain
manager spec (`2026-10-02-toolchain-manager-design.md` §2, §4–§6, §12, §15). Every observed
value below comes from experiment 115 §1 (`docs/research/experiment-backlog.md`, end of
file). Read the spec and §1 before starting.

## Global Constraints

- `CLAUDE.md` is binding. Library paths return `Result`; no `unwrap`/`expect`/`panic!`
  outside tests. Every `.rs` file is at most 300 lines, tests included. One type per file.
  New API is a domain type with methods. Value types never read env, argv or stdout: in
  symdev that is `Provision` (CLI), in `symdev-packages` the `*Tool` types of `pkgtools`.
- Gates before any "done", in `~/worktrees/symdev/cargo-run`: `cargo test --workspace
  --offline`, `cargo clippy --workspace --all-targets --offline` (zero warnings), `cargo fmt
  --all --check`. In `~/worktrees/symdev-packages/cargo-run`: `cargo test --locked`, `cargo
  clippy --all-targets --locked` (zero warnings), `cargo fmt --all --check`, and the shell
  tests under `tests/`. Read a gate's result from a file, never through `| tail`.
- Commit messages: one full imperative sentence ending with a period, then the attribution
  trailer the session gives. Stage files by name, never `git add -A`.
- Branches: symdev `cargo-run` in `~/worktrees/symdev/cargo-run`; symdev-packages
  `cargo-run` in `~/worktrees/symdev-packages/cargo-run` (base `3fe6a76`, not pushed);
  EKA2L1 in an own copy `~/src/EKA2L1-wt/emulator-pkg`, branch `symdev`. Implementing agents
  may push symdev's `cargo-run`, and nothing else.
- **LEAD ONLY, after the owner's explicit go** (implementing agents prepare up to these and
  stop): L1 pushing the rebuilt `symdev` branch to `4akloon/EKA2L1`, which starts its CI;
  L2 publishing `emulator;<V>` to the public bucket; L3 publishing `firmware;rm-469;1` to
  the private bucket; L4 pushing `symdev-packages`.
- Firmware, ROM, SDK, `.sis`, `.sisx`, `.cer` and `.key` files never enter git, CI or the
  public bucket. Staged firmware trees and archives live in `~/src/emu-pkg-scratch/` only.
- EKA2L1 is GPL-3.0: read it to learn its layout, never copy its code into symdev or
  `pkgtools`. Never run it without `--data-dir` (even `--help`: that rotates the owner's
  logs). Run it under the agent lock `flock ~/.local/share/EKA2L1/.symdev-agent.lock`, stop
  only PIDs you started, with `kill -9`. Never write into `~/.local/share/EKA2L1`.
- Recurring helpers (recipe builds, CI steps, checks) are Rust: `pkgtools` subcommands, reusing
  its existing types. `build.sh` / `stage.sh` stay thin bash drivers that fetch, check
  `sha256sum` and call `pkgtools`. `pkgtools` stays offline (no HTTP).
- Never invent tool argv: EKA2L1 gets exactly `--data-dir <dir> --control <socket>` (and
  `--help` for the probe), as experiment 114 observed.
- Scratch: `~/src/emu-pkg-scratch/`. Each script starts with a comment saying what it does.
- Keep `docs/research/wip/emulator-packages.md` current: facts, rulings, the exact next
  step. Commit it after every task, and at least every ~15 minutes.
- `<V>`, the emulator version, is the UTC commit date of the rebuilt integration head as
  `yyyy.mm.dd` (Task 1 step 6 computes it and writes it to the notes). Replace `<V>`
  everywhere below with that value. `<C>` is that head's full SHA-1, and `<c>` its first
  7 hex digits.

## What experiment 115 §1 changed in the spec

1. **symdev starts `usr/bin/eka2l1_qt`, not `AppRun`.** `AppRun` is a symlink to it, and a
   process started as `AppRun` has the comm `AppRun`. `device::is_eka2l1` would drop it from
   the registry. The binary's `RUNPATH $ORIGIN/../lib` and `usr/bin/qt.conf` make the tree
   self-contained, with no environment needed.
2. **The fork has never run its CI.** The workflow is `active`, yet there are zero runs and
   zero artifacts, although branches were pushed. L1 includes checking that a run starts,
   and dispatching one if none does.
3. **The corresponding source is much larger than "EKA2L1 + Qt".** The AppImage bundles
   files from 167 Ubuntu packages, among them FFmpeg, x264 and x265 (GPL-2.0+). 35 of their
   copyright files are not machine-readable. Decision D1 below.
4. **The glibc floor is 2.38.** symdev's other packages run from glibc 2.28. Finding F1
   below; the recipe records the floor and `pkgtools` checks it.
5. **EKA2L1 writes drive Z at every start** (`avkonfep.dll` → `.bak`, then a copy that fails
   on Linux). Task 6 observes a read-only firmware package before Task 7 decides between
   links and copies.
6. **Today the default `~/.local/share/EKA2L1` is read without `SYMDEV_EKA2L1_DATA`**
   (`EmulatorData::from_env`). The spec's order makes the variable the only way to the
   user's data. The default folder is then no longer read, which the acceptance's `bwrap
   --tmpfs` relies on. Task 7 deletes `from_env`.
7. **The publisher refuses unknown recipe keys** (`deny_unknown_fields`, and `commit` needs
   a `tag`). The CI facts therefore live in `artifact.toml` beside `recipe.toml`, read only
   by `build.sh`. The publisher does not change.
8. **The fork's artifact will expire** (no `retention-days`; the repository default, at
   most 90 days). The recipe's hashes are the lasting record; the package in R2 stays.

## Decision D1 (open, the owner's): what goes out with the public `emulator` package

The spec asks for "the fork commit as a `git archive` including submodules, plus the source
of the Qt version linuxdeploy bundled", with a licence field of "at least
`GPL-3.0-or-later` … and `LGPL-3.0-only`". Experiment 115 §1.2 shows the AppImage carries
more:

- 187 libraries from **167 Ubuntu 24.04 packages**. Qt's multimedia plugin pulls Ubuntu's
  FFmpeg, which pulls `libx264` and `libx265` (GPL-2.0+), `libzvbi`, `libcodec2` and others.
  Distributing their binaries obliges us to offer their corresponding source too (GPL-2.0
  §3, GPL-3.0 §6, LGPL §4/§6), not only Qt's. The exact versions are known only to the
  runner that built the AppImage.
- 96 of the 167 copyright files mention a GPL. 35 are not in the machine-readable format,
  so a complete SPDX expression cannot be computed; it needs a reading.
- `CLAUDE.md` says the built-in public source carries "only GPL/MIT packages". The bundle
  is GPL-3.0-compatible as a whole, but its parts are also LGPL, BSD, Apache-2.0, MPL-2.0,
  ISC and more.

| Option | What it means | Cost |
|---|---|---|
| **A (recommended)** | One commit on the integration branch only (not an upstream PR) adds a step to the fork CI's Linux job. The step writes `eka2l1-qt-x64.packages.tsv` into the artifact: for each bundled library and plugin, its Ubuntu package, version, source package and source version. The recipe fetches each source package at that exact version from Launchpad and checks it against its `.dsc`. It packs them with EKA2L1's `git archive` (submodules included) and the recipe directory into the one `--source-code` archive. Licence field: `GPL-3.0-or-later AND LGPL-3.0-only AND LicenseRef-EKA2L1-bundle`. The LicenseRef is `share/doc/eka2l1/BUNDLED.tsv` plus `usr/share/doc/<package>/copyright` in the package. `licensing.md` gets the rule that the public bucket may carry a GPL program together with the free libraries it bundles | The integration branch is "master + our PRs + one CI commit". The source archive is several hundred MB (measured in Task 11) |
| B | A's process applied to a smaller bundle. The recipe deletes the multimedia (`ffmpeg`, `gstreamer`), `networkinformation` and `tls` plugins, and the libraries only they need, before packing. Task 13 must then show that EKA2L1 still starts, draws and plays sound | Fewer sources, but the package is no longer the CI's artifact. Still needs A's list for Qt, SDL2, ICU, GLib … |
| C | `emulator;<V>` goes to the **private** bucket for 0.4.0, so there is no public distribution yet. The public package follows once A is done | Contradicts spec §3. Only machines with the private keys get `cargo run` working |

**Recommendation: A.** It is the only option that ships exactly what the CI built and
meets every source obligation by construction: the build itself records the versions.
B saves storage but changes the artifact. C postpones the problem.

**Tasks that depend on D1:** Task 2 and Task 11 run only for A (B adds a pruning step to
Task 10 and keeps Tasks 2 and 11). Task 10's licence field and Task 14's `licensing.md`
text follow the choice. Everything else is the same for A, B and C. Until the owner
decides, implement A and stop before L1.

## Finding F1 (for the owner): the emulator needs glibc 2.38

The AppImage's libraries need `GLIBC_2.38` (Ubuntu 24.04+, Debian 13+, Fedora 39+, RHEL 10).
symdev itself is static and GCCE needs glibc 2.28. The spec says a newer floor is "a finding
for the owner, not a silent rebuild". The plan records the floor in `artifact.toml`, and
`pkgtools emulator-check` fails if the tree needs anything newer. `README.md` states the
requirement. On an older host, the loader's `GLIBC_2.38 not found` reaches the user through
`EmulatorInstance::start`'s error, which quotes the emulator's last output lines. The fix
for older hosts (building on an older base) is a later decision.

## Steps reserved for the lead

Each needs the owner's explicit go. Implementing agents stop before them and say so in
the notes.

- **L1** (after Task 15): push `~/src/EKA2L1-wt/emulator-pkg` branch `symdev` to
  `4akloon/EKA2L1` (`git push --force-with-lease fork symdev`). Check that a `C/C++ CI` run
  starts for `<C>` (`gh run list -R 4akloon/EKA2L1 --branch symdev`). If none starts within
  five minutes, enable Actions for the fork in its web UI and run `gh workflow run
  build.yml -R 4akloon/EKA2L1 --ref symdev`. Wait until the `build-desktop (linux)` job is
  green, then hand Task 16 its run id.
- **L2** (after Task 16): `emulator;<V>` reaches the public bucket by merging the
  `symdev-packages` branch into `main`. `.github/workflows/emulator.yml` (Task 12) then
  publishes it. Check the cross-repository artifact download in the PR run first (Task 12
  notes why it may need a token).
- **L3** (after Task 16): on the owner's machine, with the publisher keys:
  `EKA2L1_DATA=~/.local/share/EKA2L1/data bash recipes/firmware/rm-469/1/stage.sh
  ~/src/emu-pkg-scratch/firmware/tree`, then `cargo run --release -p publish -- private
  'firmware;rm-469;1' --from ~/src/emu-pkg-scratch/firmware/tree --recipe
  recipes/firmware/rm-469/1/recipe.toml`.
- **L4**: push `~/worktrees/symdev-packages/cargo-run` and open its PR (this is what L2
  merges).
- After L2 and L3: the real-bucket acceptance (end of this plan).

## Review Focus

These are the five inputs the spec implies and that no task's main tests cover, most likely
first. Each has its test in the task named.

1. **The user's shell exports `LD_LIBRARY_PATH` or `QT_PLUGIN_PATH`.** The owner's own
   wrapper does this for his host build. Inherited, these load the host's Qt 6.8 into the
   bundled Qt 6.4 and the emulator fails at start. Expected: a packaged EKA2L1 starts
   without the host's library and plugin paths; the user's own (`SYMDEV_EKA2L1`) keeps its
   whole environment (Task 8, tests `a_packaged_eka2l1_does_not_inherit_the_hosts_library_paths`
   and `the_users_eka2l1_keeps_its_environment`).
2. **`SYMDEV_EKA2L1` still names an EKA2L1 without `--control`.** The owner's
   `~/.local/bin/eka2l1` is one. Expected: the error names the variable and says that
   unsetting it makes symdev use the `emulator` package (Task 9, test
   `an_old_symdev_eka2l1_is_named_with_the_way_to_the_package`).
3. **The firmware package is uninstalled or replaced under an existing profile.** The
   profile's ROM and Z links then point at nothing. Expected: refused before EKA2L1 starts,
   naming the profile, the missing path and the install command (Task 7, test
   `a_profile_whose_package_is_gone_is_refused_before_start`).
4. **A profile already exists, with no source configured or with `--offline`.** Expected:
   `symdev devices` and a start use it and install nothing (Task 9, test
   `an_existing_profile_needs_no_firmware_package`).
5. **Firmware sits in the default `~/.local/share/EKA2L1` and `SYMDEV_EKA2L1_DATA` is unset.**
   Expected: that folder is neither read nor written, and the error names
   `SYMDEV_EKA2L1_DATA` as the way to use it (Task 9, test
   `the_default_eka2l1_folder_is_not_read_without_symdev_eka2l1_data`).

## File structure

symdev (`~/worktrees/symdev/cargo-run`):

| Path | Responsibility |
|---|---|
| `crates/symdev-manifest/src/schema.rs` | `Device::ALL`: every device symdev supports |
| `crates/symdev-sdk/src/pins.rs` | `Pins::emulator()`, `Pins::firmware(device)` |
| `crates/symdev-sdk/src/emulator_package.rs` | `EmulatorPackage`: an installed `emulator;…`, its program |
| `crates/symdev-sdk/src/firmware_package.rs` | `FirmwarePackage`: an installed `firmware;<fw>;…`, its layout |
| `crates/symdev-sdk/src/catalog.rs` | the "way around a source" hint for `sdk`, `emulator`, `firmware` ids |
| `crates/symdev-emulator/src/device/firmware.rs` | `Firmware`: the user's EKA2L1 data or a firmware package |
| `crates/symdev-emulator/src/device/emulator_profile.rs` | `create(&Firmware)`, `check()` |
| `crates/symdev-emulator/src/device/profile_files.rs` | the file helpers moved out of the profile (copy, link, make) |
| `crates/symdev-emulator/src/device/eka2l1.rs` | `Eka2l1`: the user's EKA2L1 or the package's program, and its environment |
| `crates/symdev-emulator/src/device/emulator_instance.rs` | start and probe an `Eka2l1` |
| `crates/symdev-emulator/src/lib.rs`, `results.rs` | `Eka2l1Backend` and `EmulatorData::from_env` deleted |
| `crates/symdev-cli/src/provision/emulator.rs` | `Provision::eka2l1()`, `Provision::firmwares()` |
| `crates/symdev-cli/src/devices_cmd.rs`, `run.rs`, `run/device_pick.rs`, `test_cmd.rs`, `main.rs` | pass `&Provision` through |
| `crates/symdev-cli/tests/emulator_packages.rs` | the CLI against a `file://` source with the two packages |
| `README.md`, `crates/symdev-emulator/README.md`, `docs/research/licensing.md` | requirements, rules |

symdev-packages (`~/worktrees/symdev-packages/cargo-run`):

| Path | Responsibility |
|---|---|
| `pkgtools/src/device_entry.rs` | `DeviceEntry`: one device of an EKA2L1 `devices.yml` |
| `pkgtools/src/emulator_tree.rs` (+ `emulator_tree/glibc.rs`) | `EmulatorTree`: the extracted AppImage's layout and glibc floor |
| `pkgtools/src/emulator_notices.rs` | `EmulatorNotices`: `share/doc/eka2l1/` from the source tree and the package list |
| `pkgtools/src/dsc.rs` | `Dsc`: a Debian source control file's files and SHA-256s (D1 = A) |
| `recipes/firmware/rm-469/1/{recipe.toml,stage.sh}` | the private firmware package |
| `recipes/emulator/<V>/{recipe.toml,artifact.toml,build.sh,source.sh}` | the public emulator package |
| `.github/workflows/emulator.yml` | PR: build and dry-run; `main`: publish |
| `tests/firmware-stage.test`, `tests/emulator-build.test` | the drivers against fake inputs |

EKA2L1 (outside git of symdev): `~/src/EKA2L1-wt/emulator-pkg` (branch `symdev`), notes
`~/src/EKA2L1-wt/emulator-pkg.NOTES.md`, host build `~/src/EKA2L1-wt-build/emulator-pkg`.

---

## Phase A — everything before the fork's CI runs

### Task 1: The integration branch, rebuilt on upstream `master` with our 11 PRs

**Files:**
- Outside git: `~/src/EKA2L1-wt/emulator-pkg` (a copy of the integration clone, branch
  `symdev` rebuilt), `~/src/EKA2L1-wt/emulator-pkg.NOTES.md`, host build
  `~/src/EKA2L1-wt-build/emulator-pkg`, wrapper `~/src/emu-pkg-scratch/bin/eka2l1-emupkg`
- Modify: `docs/research/wip/emulator-packages.md` (heads, conflicts, `<V>`, `<C>`)

**Interfaces:**
- Consumes: the PR heads of experiment 115 §1.1.
- Produces: local branch `symdev` at `<C>` = upstream `master` + the 11 PR merges; `<V>`;
  an EKA2L1 host build with `--data-dir` and `--control` for quick checks.

- [ ] **Step 1: Make the copy and fetch** (the `eka2l1-host` skill applies; the clone is
  shallow, so deepen if a merge finds no base)

```bash
cp -a --reflink=auto ~/src/EKA2L1-wt/integration ~/src/EKA2L1-wt/emulator-pkg
cd ~/src/EKA2L1-wt/emulator-pkg
git fetch origin master
git fetch fork fix/command-list-overflow fix/cli-install-then-run \
  fix/property-cancel-during-wipeout fix/applist-no-localisable-rsc dev/data-dir \
  dev/anim-window-lifetime dev/applist-reload dev/applist-lock dev/control-server \
  dev/control-input dev/control-events
gh pr list -R EKA2L1/EKA2L1 --author 4akloon --state open --json number,headRefName,headRefOid
```

Write the PR list and each fetched head into `emulator-pkg.NOTES.md`. A head that differs
from experiment 115 §1.1 is used as fetched (the PR moved); note the old and new SHA. A PR
merged upstream since is left out. A new open PR of ours is not added: stop and ask the
lead (the spec names eleven).

- [ ] **Step 2: Rebuild the branch**

```bash
git branch symdev-d07d5ac symdev          # the old integration head, kept for reference
git checkout -B symdev origin/master
for b in fix/command-list-overflow fix/cli-install-then-run fix/property-cancel-during-wipeout \
         fix/applist-no-localisable-rsc dev/data-dir dev/anim-window-lifetime \
         dev/applist-reload dev/applist-lock dev/control-events; do
  git merge --no-ff --no-edit -m "Merge $b ($(git rev-parse --short fork/$b)) into symdev." "fork/$b" || break
done
git submodule update --init --recursive
```

`dev/control-events` carries `dev/control-server` and `dev/control-input`. Check with
`git merge-base --is-ancestor fork/dev/control-server fork/dev/control-events`. On a
conflict the loop stops. Resolve it by keeping both sides' behaviour; the known one is the
option lists in `src/emu/qt/src/thread.cpp` and `src/emu/qt/include/qt/cmdhandler.h`, where
both options stay. Then `git commit --no-edit` and rerun the loop from the next branch.
Record every conflict, file and resolution in the NOTES. If git says "refusing to merge
unrelated histories", run `git fetch --deepen=1000 origin master` and the PR branches, then
start the step again.

- [ ] **Step 3: Build and test on this host**

`~/src/EKA2L1-wt-build/emulator-pkg/build.sh` is `~/src/EKA2L1-wt-build/cargo-run/build.sh`
with `cargo-run` replaced by `emulator-pkg` everywhere (`sed s/cargo-run/emulator-pkg/g`) and
`ekatests` added to its `ninja` targets. It configures once (the same cache options as the
integration build, `-DEKA2L1_BUILD_TESTS=ON`), builds, restores `src/emu/qt/translations`
and appends `EXIT=<rc>` to its `build.log`. Then:

```bash
mkdir -p ~/src/EKA2L1-wt-build/emulator-pkg/tmp
bash ~/src/EKA2L1-wt-build/emulator-pkg/build.sh; tail -1 ~/src/EKA2L1-wt-build/emulator-pkg/build.log
(cd ~/src/EKA2L1-wt-build/emulator-pkg/src/tests && ./ekatests) > ~/src/emu-pkg-scratch/ekatests.log 2>&1
tail -2 ~/src/emu-pkg-scratch/ekatests.log
```

Expected: `EXIT=0`, and `ekatests.log` ends with `All tests passed`. A
failure that the merges caused is fixed on the branch as its own merge-fix commit, with the
reason in the NOTES. A failure that `origin/master` alone also shows is recorded and left
alone.

- [ ] **Step 4: Check both options**

`~/src/emu-pkg-scratch/bin/eka2l1-emupkg` is `~/src/cargo-run-scratch/bin/eka2l1-symdev`
with its `exec` line pointing at `~/src/EKA2L1-wt-build/emulator-pkg/bin/eka2l1_qt`. Under
the agent lock, with a scratch home (help never exits, and without `--data-dir` it rotates
the owner's logs):

```bash
H=~/src/emu-pkg-scratch/help; rm -rf $H; mkdir -p $H
flock ~/.local/share/EKA2L1/.symdev-agent.lock env HOME=$H XDG_DATA_HOME=$H/share \
  XDG_CONFIG_HOME=$H/config XDG_CACHE_HOME=$H/cache XAUTHORITY=${XAUTHORITY:-$HOME/.Xauthority} \
  timeout -s KILL 15 ~/src/emu-pkg-scratch/bin/eka2l1-emupkg --data-dir $H/data --help > $H/help.txt 2>&1
grep -E -- '^ *--(control|data-dir)' $H/help.txt
```

Expected: two lines, one for `--control` and one for `--data-dir`.

- [ ] **Step 5: Do not push**

The push is L1. Leave `fork/symdev` as it is.

- [ ] **Step 6: Record `<C>` and `<V>`**

```bash
cd ~/src/EKA2L1-wt/emulator-pkg
git rev-parse symdev                                                   # <C>
TZ=UTC git log -1 --format=%cd --date=format-local:%Y.%m.%d symdev     # <V>
```

Write both into the NOTES and into `docs/research/wip/emulator-packages.md`. Commit the
wip file: `Record the rebuilt EKA2L1 integration branch and the emulator version it gives.`

### Task 2: (D1 = A) The fork CI lists the Ubuntu packages its AppImage bundles

**Files:**
- Modify (EKA2L1 copy, branch `symdev`): `.github/workflows/build.yml`

**Interfaces:**
- Produces: the Linux artifact `eka2l1-<c>-linux` holds `eka2l1-qt-x64.AppImage` and
  `eka2l1-qt-x64.packages.tsv`. Each line of the TSV is `<binary package>\t<version>\t<source
  package>\t<source version>`, sorted, one per package that owns a file under
  `usr/lib` or `usr/plugins` of the AppDir.

- [ ] **Step 1: Add the step after "Generate AppImage"**

```yaml
    # symdev integration branch only, not an upstream change: the Ubuntu packages whose
    # files linuxdeploy put into the AppImage, with their source packages, so that their
    # corresponding source can be published with it. A bundled file that belongs to no
    # package fails the job.
    - name: List the packages the AppImage bundles
      if: matrix.label == 'linux'
      shell: bash
      run: |
        cd build/eka2l1.AppDir/usr
        find lib plugins -type f -name '*.so*' -printf '%f\n' | sort -u > ../../bundled-files.txt
        : > ../../bundled-owners.txt
        while read -r name; do
          owner=$(dpkg -S "*/$name" 2>/dev/null | awk -F': ' 'NR == 1 { print $1 }') || true
          if [ -z "$owner" ]; then echo "::error::$name belongs to no package"; exit 1; fi
          echo "$owner" >> ../../bundled-owners.txt
        done < ../../bundled-files.txt
        sort -u ../../bundled-owners.txt | xargs dpkg-query -W \
          -f '${binary:Package}\t${Version}\t${source:Package}\t${source:Version}\n' \
          | sort > ../../eka2l1-qt-x64.packages.tsv
        wc -l ../../eka2l1-qt-x64.packages.tsv
```

And the Linux upload takes both files (the artifact's root is their common folder `build/`):

```yaml
    - uses: actions/upload-artifact@v7
      with:
        name: eka2l1-${{ steps.git_short_sha.outputs.value }}-${{ matrix.label }}
        path: |
          build/eka2l1-qt-x64.AppImage
          build/eka2l1-qt-x64.packages.tsv
      if: matrix.label == 'linux'
```

- [ ] **Step 2: Check the YAML and commit on `symdev`**

```bash
cd ~/src/EKA2L1-wt/emulator-pkg
python3 -c 'import yaml; yaml.safe_load(open(".github/workflows/build.yml")); print("ok")'
git add .github/workflows/build.yml
git commit -m "ci: List the Ubuntu packages the Linux AppImage bundles (symdev integration only)."
```

Expected: `ok`. Task 3 runs the step for real.

- [ ] **Step 3: Record the new `<C>` and `<V>`** with Task 1 step 6's two commands (the head
  moved). Commit the wip file.

### Task 3: The fork CI's Linux job, rehearsed in `ubuntu:24.04`

The push (L1) makes a public CI run. A rehearsal finds build failures first: our PRs were
built against Qt 6.8.3, and the CI uses Ubuntu's 6.4.2. It also gives Tasks 6–15 an
AppImage with `--control` before any push.

**Files:**
- Outside git: `~/src/emu-pkg-scratch/rehearsal/{run.sh,run.log,out/}`
- Modify: `docs/research/experiment-backlog.md` (experiment 115 §2: the rehearsal)

**Interfaces:**
- Consumes: branch `symdev` at `<C>` (Tasks 1–2).
- Produces: `~/src/emu-pkg-scratch/rehearsal/out/eka2l1-qt-x64.AppImage`, its
  `.packages.tsv` (D1 = A) and `SHA256SUMS`. Tasks 10–15 call this "the rehearsal AppImage".

- [ ] **Step 1: Write `run.sh`**

```bash
#!/usr/bin/env bash
# run.sh — rehearse build.yml's build-desktop (linux) job on the symdev branch in ubuntu:24.04:
# the job's apt list, cmake flags, build, ctest, generate_appimage.sh and (D1 = A) the
# package list. The extra apt packages are what the GitHub runner image has preinstalled.
set -euo pipefail
R=~/src/emu-pkg-scratch/rehearsal
rm -rf "$R/src" "$R/out"; mkdir -p "$R/out"
git clone --quiet --branch symdev ~/src/EKA2L1-wt/emulator-pkg "$R/src"
git -C "$R/src" submodule update --init --recursive --quiet
docker run --rm -v "$R/src:/src" -w /src -e DEBIAN_FRONTEND=noninteractive \
  -e APPIMAGE_EXTRACT_AND_RUN=1 -e QMAKE=/usr/bin/qmake6 ubuntu:24.04 bash -euo pipefail -c '
  apt-get update
  apt-get -y install ccache libgtk-3-dev libpulse-dev libasound2-dev libsdl2-dev pulseaudio \
    qt6-base-dev qt6-base-private-dev qt6-tools-dev qt6-tools-dev-tools qt6-l10n-tools \
    libqt6svg6-dev qt6-multimedia-dev \
    build-essential cmake ninja-build git wget curl file python3 pkg-config ca-certificates
  cmake -B build -DCI=ON -DEKA2L1_ENABLE_UNEXPECTED_EXCEPTION_HANDLER=ON -DEKA2L1_NO_TERMINAL=ON \
    -DEKA2L1_ENABLE_DISCORD_RICH_PRESENCE=ON -DCMAKE_BUILD_TYPE=Release
  cmake --build build --config Release --parallel 16 --target eka2l1_qt ekatests
  ctest --test-dir build -C Release --output-on-failure
  chmod u+x scripts/generate_appimage.sh && ./scripts/generate_appimage.sh
  bash .github/rehearse-package-list.sh
  chown -R '"$(id -u):$(id -g)"' /src'
cp "$R/src/build/eka2l1-qt-x64.AppImage" "$R/src/build/eka2l1-qt-x64.packages.tsv" "$R/out/"
(cd "$R/out" && sha256sum eka2l1-qt-x64.AppImage eka2l1-qt-x64.packages.tsv > SHA256SUMS)
```

`.github/rehearse-package-list.sh` does not exist in the branch. Before the run, copy the
`run:` block of Task 2's step into `$R/src/.github/rehearse-package-list.sh` with `set -euo
pipefail` on top, as the clone is the rehearsal's own. Without D1 = A, drop that line and
the `.packages.tsv` from the copies.

- [ ] **Step 2: Run it in the background and wait for the end**

```bash
bash ~/src/emu-pkg-scratch/rehearsal/run.sh > ~/src/emu-pkg-scratch/rehearsal/run.log 2>&1; echo "EXIT=$?" >> ~/src/emu-pkg-scratch/rehearsal/run.log
```

Expected: `EXIT=0`. On a missing build tool (cmake or FFmpeg's configure names it), add the
Ubuntu package to the second apt line: it is in the runner image too, not a workflow
change. Record each added package. On a compile error in our code against Qt 6.4.2, fix it
on the PR branch it comes from, in the fork's PR copy (`~/src/EKA2L1-wt/<topic>`, the
`eka2l1-host` skill), and repeat Task 1 from step 2. That PR then needs a push, which the
lead does with L1; record it in the NOTES.

- [ ] **Step 3: Look at what it made** (experiment 115 §2)

```bash
cd ~/src/emu-pkg-scratch/rehearsal && rm -rf x && mkdir x && cd x
../out/eka2l1-qt-x64.AppImage --appimage-extract > /dev/null
ls -la squashfs-root; readlink squashfs-root/AppRun; cat squashfs-root/usr/bin/qt.conf
objdump -p squashfs-root/usr/bin/eka2l1_qt | grep RUNPATH
find squashfs-root -type f -exec sh -c 'objdump -T "$1" 2>/dev/null' _ {} \; \
  | grep -o 'GLIBC_[0-9.]*' | sort -uV | tail -1
du -sh squashfs-root; wc -l ../out/eka2l1-qt-x64.packages.tsv
```

`--appimage-extract` runs only the AppImage runtime, which unpacks the tree; EKA2L1 does
not start. Expected, as in §1.2: `AppRun -> usr/bin/eka2l1_qt`, `Plugins = plugins`,
`RUNPATH $ORIGIN/../lib`, and the floor `GLIBC_2.38`. Record all of it, with `run.log`'s
build time and the added packages, as experiment 115 §2. A difference from §1.2 (an
`apprun-hooks/` directory, another floor) is recorded and reported to the lead before
Task 8: it changes how symdev starts the program.

- [ ] **Step 4: Commit** `docs/research/experiment-backlog.md` and the wip file:
  `Record experiment 115 §2: the fork CI's Linux job rehearsed on the rebuilt branch.`

### Task 4: The two pins, the two package layouts, and the way around the sources

**Files:**
- Modify: `crates/symdev-manifest/src/schema.rs` (`Device::ALL`)
- Modify: `crates/symdev-sdk/src/pins.rs` (`emulator`, `firmware`, tests)
- Create: `crates/symdev-sdk/src/emulator_package.rs`, `crates/symdev-sdk/src/firmware_package.rs`
- Modify: `crates/symdev-sdk/src/lib.rs` (modules and re-exports)
- Modify: `crates/symdev-sdk/src/catalog.rs` (`bypass` replaces the `sdk: bool`)
- Create: `crates/symdev-sdk/src/manager/tests/bypass.rs`; Modify: `crates/symdev-sdk/src/manager/tests.rs` (`mod bypass;`)

**Interfaces:**
- Produces:
  - `Device::ALL: [Device; 1]`.
  - `Pins::emulator() -> PackageId` (`emulator;<V>`), `Pins::firmware(Device) -> PackageId`
    (`firmware;rm-469;1` for `NokiaE52`).
  - `EmulatorPackage::at(root: PathBuf, id: &PackageId) -> Result<EmulatorPackage>`,
    `EmulatorPackage::PROGRAM = "usr/bin/eka2l1_qt"`, `.program() -> PathBuf`.
  - `FirmwarePackage::at(root: PathBuf, id: &PackageId) -> Result<FirmwarePackage>`,
    `.root() -> &Path`, `.name() -> &str` (the id's second segment, `rm-469`).
  - A lookup failure for an `emulator` id names `SYMDEV_EKA2L1`, for a `firmware` id
    `SYMDEV_EKA2L1_DATA`, as an `sdk` id names `SYMDEV_EPOCROOT` today.

- [ ] **Step 1: Write the failing tests**

In `pins.rs`'s test module (keep the existing ones; add `Pins::emulator()` and
`Pins::firmware(Device::NokiaE52)` to `every_pin_is_a_valid_id`'s array):

```rust
    #[test]
    fn the_e52_runs_on_the_rm_469_firmware() {
        assert_eq!(
            Pins::firmware(Device::NokiaE52),
            PackageId::parse("firmware;rm-469;1").unwrap()
        );
    }

    #[test]
    fn the_emulator_is_pinned_to_a_dated_build() {
        let id = Pins::emulator();
        let segments: Vec<&str> = id.segments().collect();
        assert_eq!(segments.len(), 2, "{id}");
        assert_eq!(segments[0], "emulator");
        let date: Vec<&str> = segments[1].split('.').collect();
        let widths: Vec<usize> = date.iter().map(|part| part.len()).collect();
        assert_eq!(widths, [4, 2, 2], "{id}");
        assert!(date.iter().all(|p| p.chars().all(|c| c.is_ascii_digit())), "{id}");
    }

    #[test]
    fn every_device_has_a_firmware() {
        for device in Device::ALL {
            assert_eq!(Pins::firmware(device).kind(), "firmware");
        }
    }
```

`crates/symdev-sdk/src/emulator_package.rs` and `firmware_package.rs` start as these test
modules only; declare `mod emulator_package;` and `mod firmware_package;` in `lib.rs` now
so that they compile. The first gets a test module like `platform_sdk.rs`'s:

```rust
#[cfg(test)]
mod tests {
    use std::fs;

    use super::EmulatorPackage;
    use crate::PackageId;

    fn id(s: &str) -> PackageId {
        PackageId::parse(s).unwrap()
    }

    #[test]
    fn the_program_is_the_binary_apprun_links_to() {
        let tmp = tempfile::tempdir().unwrap();
        fs::create_dir_all(tmp.path().join("usr/bin")).unwrap();
        fs::write(tmp.path().join("usr/bin/eka2l1_qt"), b"").unwrap();
        let p = EmulatorPackage::at(tmp.path().to_path_buf(), &id("emulator;2026.10.04")).unwrap();
        assert_eq!(p.program(), tmp.path().join("usr/bin/eka2l1_qt"));
    }

    #[test]
    fn a_missing_program_is_named_with_the_reinstall_command() {
        let tmp = tempfile::tempdir().unwrap();
        let e = EmulatorPackage::at(tmp.path().to_path_buf(), &id("emulator;2026.10.04"))
            .unwrap_err()
            .to_string();
        assert!(e.starts_with(&tmp.path().join("usr/bin/eka2l1_qt").display().to_string()), "{e}");
        assert!(e.contains("symdev sdk uninstall 'emulator;2026.10.04' && symdev sdk install 'emulator;2026.10.04'"), "{e}");
    }

    #[test]
    fn refuses_an_id_that_is_not_an_emulator() {
        let tmp = tempfile::tempdir().unwrap();
        let e = EmulatorPackage::at(tmp.path().to_path_buf(), &id("gcce;12.1.0")).unwrap_err();
        assert!(e.to_string().contains("gcce;12.1.0"), "{e}");
    }
}
```

`crates/symdev-sdk/src/firmware_package.rs`:

```rust
#[cfg(test)]
mod tests {
    use std::fs;

    use super::FirmwarePackage;
    use crate::PackageId;

    fn id(s: &str) -> PackageId {
        PackageId::parse(s).unwrap()
    }

    fn tree(root: &std::path::Path) {
        fs::create_dir_all(root.join("roms/rm-469")).unwrap();
        fs::create_dir_all(root.join("drives/z/rm-469")).unwrap();
        fs::write(root.join("device.yml"), "RM-469:\n  firmcode: RM-469\n").unwrap();
    }

    #[test]
    fn the_firmware_is_named_by_the_ids_second_segment() {
        let tmp = tempfile::tempdir().unwrap();
        tree(tmp.path());
        let f = FirmwarePackage::at(tmp.path().to_path_buf(), &id("firmware;rm-469;1")).unwrap();
        assert_eq!(f.name(), "rm-469");
        assert_eq!(f.root(), tmp.path());
    }

    #[test]
    fn each_missing_part_is_named_with_the_reinstall_command() {
        for part in ["device.yml", "roms/rm-469", "drives/z/rm-469"] {
            let tmp = tempfile::tempdir().unwrap();
            tree(tmp.path());
            let gone = tmp.path().join(part);
            if gone.is_dir() { fs::remove_dir_all(&gone).unwrap() } else { fs::remove_file(&gone).unwrap() }
            let e = FirmwarePackage::at(tmp.path().to_path_buf(), &id("firmware;rm-469;1"))
                .unwrap_err()
                .to_string();
            assert!(e.starts_with(&gone.display().to_string()), "{part}: {e}");
            assert!(e.contains("symdev sdk install 'firmware;rm-469;1'"), "{part}: {e}");
        }
    }

    #[test]
    fn refuses_an_id_that_is_not_a_firmware_of_three_segments() {
        let tmp = tempfile::tempdir().unwrap();
        tree(tmp.path());
        for bad in ["sdk;s60-3rd-fp2;1.1", "firmware;1", "firmware;rm-469;1;x"] {
            let e = FirmwarePackage::at(tmp.path().to_path_buf(), &id(bad)).unwrap_err();
            assert!(e.to_string().contains("firmware;<firmware>;<n>"), "{bad}: {e}");
        }
    }
}
```

`crates/symdev-sdk/src/manager/tests/bypass.rs` (and `mod bypass;` in `manager/tests.rs`):

```rust
//! The way around the sources that a failed lookup names, per kind of package.

use super::repo::Repo;
use super::{id, keyless_private, manager};
use crate::Host;

#[test]
fn an_emulator_found_nowhere_names_symdev_eka2l1_and_sources_toml() {
    let tmp = tempfile::tempdir().unwrap();
    let mut repo = Repo::new(tmp.path().join("repo"));
    repo.add("gcce;12.1.0", Host::X86_64Linux, &[]);
    let mut progress = Vec::new();
    let e = manager(&tmp, vec![repo.source("public")], false, &mut progress)
        .ensure(&[id("emulator;2026.10.04")])
        .unwrap_err()
        .to_string();
    assert_eq!(
        e,
        "emulator;2026.10.04 was not found in the sources searched: `public`; set \
         SYMDEV_EKA2L1 to your own EKA2L1 with --control and --data-dir, or add a source \
         that has it in /config/symdev/sources.toml"
    );
}

#[test]
fn a_firmware_behind_a_keyless_private_source_names_the_keys_and_symdev_eka2l1_data() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = Repo::new(tmp.path().join("repo"));
    repo.write_index();
    let sources = vec![repo.source("public"), keyless_private()];
    let mut progress = Vec::new();
    let e = manager(&tmp, sources, false, &mut progress)
        .ensure(&[id("firmware;rm-469;1")])
        .unwrap_err()
        .to_string();
    assert!(e.contains("SYMDEV_SOURCE_PRIVATE_ACCESS_KEY_ID"), "{e}");
    assert!(
        e.contains("or set SYMDEV_EKA2L1_DATA to an EKA2L1 data folder that has this firmware installed"),
        "{e}"
    );
    assert_eq!(e.matches("SYMDEV_EKA2L1_DATA").count(), 1, "{e}");
}

#[test]
fn a_gcce_found_nowhere_names_no_variable() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = Repo::new(tmp.path().join("repo"));
    repo.write_index();
    let mut progress = Vec::new();
    let e = manager(&tmp, vec![repo.source("public")], false, &mut progress)
        .ensure(&[id("gcce;99.0")])
        .unwrap_err()
        .to_string();
    assert!(!e.contains("SYMDEV_"), "{e}");
}
```

`Repo::new` writes no index; `write_index()` writes the empty one, so the source is
searched rather than reported unreadable.

- [ ] **Step 2: Run them and see them fail**

```bash
cargo test -p symdev-sdk --offline -- pins:: emulator_package firmware_package bypass > /tmp/t4.log 2>&1; grep -E "^error|test result" /tmp/t4.log
```

Expected: compile errors (`no function or associated item named emulator`,
`unresolved import super::EmulatorPackage`, `no associated item named ALL`). libtest takes several
filters after `--` and runs what matches any of them.

- [ ] **Step 3: Implement**

`crates/symdev-manifest/src/schema.rs`, below `enum Device`:

```rust
impl Device {
    /// Every device symdev supports: what is made once per device (emulator profiles)
    /// iterates over it.
    pub const ALL: [Device; 1] = [Device::NokiaE52];
}
```

`crates/symdev-sdk/src/pins.rs`, inside `impl Pins` (`<V>` from Task 1):

```rust
    /// The EKA2L1 `cargo run` starts when `SYMDEV_EKA2L1` is not set: the fork CI's build of
    /// the integration branch this release was tested with (emulator packages spec §3).
    pub fn emulator() -> PackageId {
        PackageId::pinned("emulator;<V>")
    }

    /// The firmware an emulator profile of `device` is made from when `SYMDEV_EKA2L1_DATA`
    /// is not set (emulator packages spec §4). Only a private source has it.
    pub fn firmware(device: Device) -> PackageId {
        match device {
            Device::NokiaE52 => PackageId::pinned("firmware;rm-469;1"),
        }
    }
```

`crates/symdev-sdk/src/emulator_package.rs` (above its tests):

```rust
use std::path::PathBuf;

use crate::{PackageId, Result, SdkError};

/// An installed `emulator;<version>` package: the tree of the fork CI's EKA2L1 AppImage,
/// extracted (emulator packages spec §3). symdev starts [`Self::PROGRAM`] itself: `AppRun`
/// is only a link to it, and a process started through the link is named `AppRun`, which
/// the device registry does not take for an EKA2L1 (experiment 115 §1.2).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EmulatorPackage {
    root: PathBuf,
}

impl EmulatorPackage {
    /// The program, relative to the package root. Its `RUNPATH` and `qt.conf` find the
    /// bundled libraries and Qt plugins without any environment.
    pub const PROGRAM: &'static str = "usr/bin/eka2l1_qt";

    /// The installed package of `id` (`emulator;…`); checks that the program is there.
    pub fn at(root: PathBuf, id: &PackageId) -> Result<EmulatorPackage> {
        if id.kind() != "emulator" {
            return Err(SdkError::Other(format!(
                "{id} is not an emulator package id (`emulator;<version>`)"
            )));
        }
        let program = root.join(Self::PROGRAM);
        if !program.is_file() {
            return Err(SdkError::Other(format!(
                "{} is missing from installed {id}; run `symdev sdk uninstall {word} && symdev \
                 sdk install {word}`",
                program.display(),
                word = id.shell_word()
            )));
        }
        Ok(EmulatorPackage { root })
    }

    pub fn program(&self) -> PathBuf {
        self.root.join(Self::PROGRAM)
    }
}
```

`crates/symdev-sdk/src/firmware_package.rs` (above its tests):

```rust
use std::path::{Path, PathBuf};

use crate::{PackageId, Result, SdkError};

/// An installed `firmware;<firmware>;<n>` package (emulator packages spec §4): one
/// firmware in EKA2L1's data layout, `roms/<firmware>/` and `drives/z/<firmware>/`, and
/// that device's entry of EKA2L1's `devices.yml` as `device.yml`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FirmwarePackage {
    root: PathBuf,
    name: String,
}

impl FirmwarePackage {
    /// The installed package of `id`; checks the three parts.
    pub fn at(root: PathBuf, id: &PackageId) -> Result<FirmwarePackage> {
        let segments: Vec<&str> = id.segments().collect();
        let ["firmware", name, _] = segments[..] else {
            return Err(SdkError::Other(format!(
                "{id} is not a firmware package id (`firmware;<firmware>;<n>`)"
            )));
        };
        for part in ["device.yml".to_string(), format!("roms/{name}"), format!("drives/z/{name}")] {
            let path = root.join(&part);
            if !path.exists() {
                return Err(SdkError::Other(format!(
                    "{} is missing from installed {id}; run `symdev sdk uninstall {word} && \
                     symdev sdk install {word}`",
                    path.display(),
                    word = id.shell_word()
                )));
            }
        }
        Ok(FirmwarePackage {
            root,
            name: name.to_string(),
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The firmware's folder name in EKA2L1's data (`rm-469`).
    pub fn name(&self) -> &str {
        &self.name
    }
}
```

`lib.rs`: `pub use emulator_package::EmulatorPackage;` and `pub use
firmware_package::FirmwarePackage;`, in the alphabetical places of the existing list.

`catalog.rs`: replace `let sdk = id.kind() == "sdk";` with `let bypass = Self::bypass(id);`.
`keys_hint` takes `bypass: Option<&str>` and appends `", or {way}"`. The final hint becomes
`if let (Some(way), false) = (bypass, keyless) { message.push_str(&format!("; {way}, or add
a source that has it in {file}")); } else if …`. `all()` passes `None`. And:

```rust
    /// The way around the sources for a package of `id`'s kind, named with every failed
    /// lookup: the user's own copy, through the variable symdev reads before the packages.
    fn bypass(id: &PackageId) -> Option<&'static str> {
        match id.kind() {
            "sdk" => Some("set SYMDEV_EPOCROOT to your own SDK"),
            "emulator" => Some("set SYMDEV_EKA2L1 to your own EKA2L1 with --control and --data-dir"),
            "firmware" => Some(
                "set SYMDEV_EKA2L1_DATA to an EKA2L1 data folder that has this firmware installed",
            ),
            _ => None,
        }
    }
```

The `sdk` wording is unchanged, so `manager/tests/lookup.rs` passes as it is.

- [ ] **Step 4: Run the tests and the gates**

```bash
cargo test --workspace --offline > /tmp/t4-all.log 2>&1; grep -cE "^test result: ok" /tmp/t4-all.log; grep -E "FAILED|^error" /tmp/t4-all.log
cargo clippy --workspace --all-targets --offline > /tmp/t4-clippy.log 2>&1; grep -cE "^(warning|error)" /tmp/t4-clippy.log
cargo fmt --all --check
```

Expected: no `FAILED` or `error` line, clippy count `0`, fmt silent. `Pins::emulator` and the
two package types are not yet used outside tests. A `dead_code` warning cannot appear on a
`pub` item of a library crate; if one does, the item was made private by mistake.

- [ ] **Step 5: Commit**

```bash
git add crates/symdev-manifest/src/schema.rs crates/symdev-sdk/src/pins.rs \
  crates/symdev-sdk/src/emulator_package.rs crates/symdev-sdk/src/firmware_package.rs \
  crates/symdev-sdk/src/lib.rs crates/symdev-sdk/src/catalog.rs \
  crates/symdev-sdk/src/manager/tests.rs crates/symdev-sdk/src/manager/tests/bypass.rs
git commit -m "Pin the emulator and the E52 firmware, read their package layouts, and name SYMDEV_EKA2L1 or SYMDEV_EKA2L1_DATA when no source has them."
```

### Task 5: The private firmware recipe and `pkgtools device-entry`

All in `~/worktrees/symdev-packages/cargo-run`. The staged tree and the archive live in
`~/src/emu-pkg-scratch/firmware/`, never in the worktree.

**Files:**
- Create: `pkgtools/src/device_entry.rs` (`DeviceEntry`, with its tests)
- Modify: `pkgtools/src/main.rs` (subcommand `device-entry`)
- Create: `recipes/firmware/rm-469/1/recipe.toml`, `recipes/firmware/rm-469/1/stage.sh`
- Create: `tests/firmware-stage.test`; Modify: `.github/workflows/tests.yml` (run it)

**Interfaces:**
- Produces: `pkgtools device-entry <devices.yml> <firmcode>` prints that device's entry
  (its key line and indented lines) and exits 0; 1 with `error: …` otherwise.
  `stage.sh <out>` makes `<out>/{device.yml,roms/rm-469/,drives/z/rm-469/}`. The recipe pins
  the archive's SHA-256.

- [ ] **Step 1: Write the failing tests** in `pkgtools/src/device_entry.rs`

```rust
#[cfg(test)]
mod tests {
    use super::DeviceEntry;

    const TWO: &str = "RM-469:\n  platver: epoc93fp2\n  manufacturer: Nokia\n  firmcode: RM-469\n  model: N00\n  machine-uid: 0\n  isolated-drives: false\nRM-356:\n  platver: epoc94\n  firmcode: RM-356\n";

    #[test]
    fn takes_one_device_with_its_indented_lines() {
        let e = DeviceEntry::find(TWO, "RM-469").unwrap();
        assert_eq!(
            e.text(),
            "RM-469:\n  platver: epoc93fp2\n  manufacturer: Nokia\n  firmcode: RM-469\n  model: N00\n  machine-uid: 0\n  isolated-drives: false\n"
        );
    }

    #[test]
    fn the_last_device_ends_at_the_end_of_the_file() {
        let e = DeviceEntry::find(TWO, "RM-356").unwrap();
        assert_eq!(e.text(), "RM-356:\n  platver: epoc94\n  firmcode: RM-356\n");
    }

    #[test]
    fn a_missing_device_lists_the_ones_there() {
        let e = DeviceEntry::find(TWO, "RM-1").unwrap_err().to_string();
        assert!(e.contains("no device RM-1"), "{e}");
        assert!(e.contains("RM-469, RM-356"), "{e}");
    }

    #[test]
    fn an_entry_whose_firmcode_differs_is_refused() {
        let text = "RM-469:\n  firmcode: RM-470\n";
        let e = DeviceEntry::find(text, "RM-469").unwrap_err().to_string();
        assert!(e.contains("firmcode RM-470"), "{e}");
    }
}
```

And a CLI test in `pkgtools/tests/cli.rs`:

```rust
#[test]
fn device_entry_prints_the_entry_and_exits_1_without_it() {
    let tmp = tempfile::tempdir().unwrap();
    let yml = tmp.path().join("devices.yml");
    fs::write(&yml, "RM-469:\n  firmcode: RM-469\n").unwrap();
    let ok = pkgtools(&[&"device-entry", &yml, &"RM-469"]);
    assert_eq!(ok.status.code(), Some(0), "{}", text(&ok.stderr));
    assert_eq!(text(&ok.stdout), "RM-469:\n  firmcode: RM-469\n");
    let missing = pkgtools(&[&"device-entry", &yml, &"RM-1"]);
    assert_eq!(missing.status.code(), Some(1));
    assert!(text(&missing.stderr).starts_with("error: "), "{}", text(&missing.stderr));
}
```

- [ ] **Step 2: Run them and see them fail**

```bash
cd ~/worktrees/symdev-packages/cargo-run && cargo test --locked -p pkgtools device_entry > /tmp/t5.log 2>&1; grep -E "^error|test result" /tmp/t5.log
```

Expected: `cannot find type DeviceEntry` (declare `mod device_entry;` in `main.rs` first)
and an unknown subcommand in the CLI test.

- [ ] **Step 3: Implement** `pkgtools/src/device_entry.rs` (above its tests)

```rust
//! `DeviceEntry`: one device of EKA2L1's `devices.yml`, as the firmware recipe's stage.sh
//! takes it into the package's `device.yml` (symdev experiment 115 §1.4: a top-level key
//! per firmware code, its fields indented below it).

use crate::tool_error::{Result, ToolError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceEntry {
    text: String,
}

impl DeviceEntry {
    /// The entry whose key is `firmcode`: its key line and every indented line after it.
    /// Its own `firmcode:` field must name the same code.
    pub fn find(devices_yml: &str, firmcode: &str) -> Result<DeviceEntry> {
        let key = format!("{firmcode}:");
        let mut lines = devices_yml.lines().skip_while(|l| *l != key);
        let Some(first) = lines.next() else {
            let there: Vec<&str> = devices_yml
                .lines()
                .filter(|l| !l.starts_with([' ', '\t']) && l.ends_with(':'))
                .map(|l| l.trim_end_matches(':'))
                .collect();
            return Err(ToolError::new(format!(
                "devices.yml has no device {firmcode}; it has: {}; install that firmware in \
                 EKA2L1 first",
                there.join(", ")
            )));
        };
        let mut text = format!("{first}\n");
        for line in lines.take_while(|l| l.starts_with([' ', '\t'])) {
            text.push_str(line);
            text.push('\n');
        }
        let code = text
            .lines()
            .find_map(|l| l.trim().strip_prefix("firmcode:"))
            .map(str::trim);
        match code {
            Some(c) if c == firmcode => Ok(DeviceEntry { text }),
            Some(c) => Err(ToolError::new(format!(
                "devices.yml's entry {firmcode} has firmcode {c}"
            ))),
            None => Err(ToolError::new(format!(
                "devices.yml's entry {firmcode} has no firmcode field"
            ))),
        }
    }

    pub fn text(&self) -> &str {
        &self.text
    }
}
```

`main.rs`: `mod device_entry;`, `use crate::device_entry::DeviceEntry;` and

```rust
    /// Print one device's entry of an EKA2L1 devices.yml (the firmware recipe's
    /// device.yml). Exit 1 when it is missing or its firmcode differs.
    DeviceEntry {
        #[arg(value_name = "devices.yml")]
        devices: PathBuf,
        /// The device's key and firmware code, e.g. RM-469.
        #[arg(value_name = "firmcode")]
        firmcode: String,
    },
```

with the arm

```rust
        Command::DeviceEntry { devices, firmcode } => match std::fs::read_to_string(&devices) {
            Ok(text) => match DeviceEntry::find(&text, &firmcode) {
                Ok(entry) => {
                    let _ = out.write_all(entry.text().as_bytes());
                    0
                }
                Err(e) => {
                    let _ = writeln!(err, "error: {}: {e}", devices.display());
                    1
                }
            },
            Err(e) => {
                let _ = writeln!(err, "error: {}: {e}", devices.display());
                1
            }
        },
```

- [ ] **Step 4: Run them and see them pass**, as in step 2. Expected: `test result: ok`.

- [ ] **Step 5: Write the driver's test first**, `tests/firmware-stage.test`

```sh
#!/bin/sh
# stage.sh of firmware;rm-469;1 against a fake EKA2L1 data folder: the staged tree, the
# device.yml it takes, the refusals, and that the data folder is not written.
#
#   sh tests/firmware-stage.test
set -eu
root=$(cd "$(dirname "$0")/.." && pwd)
stage=$root/recipes/firmware/rm-469/1/stage.sh
tmp=$(mktemp -d); trap 'rm -rf "$tmp"' EXIT
cargo build --release --quiet --manifest-path "$root/Cargo.toml" -p pkgtools
export PKGTOOLS="$root/target/release/pkgtools"
failures=0
check() { name=$1; shift; if "$@"; then echo "ok - $name"; else echo "not ok - $name"; failures=$((failures + 1)); fi; }

data=$tmp/data
mkdir -p "$data/roms/rm-469" "$data/drives/z/rm-469/sys/bin" "$data/drives/z/rm-469/z:/private" "$data/drives/c/x"
printf 'rom' > "$data/roms/rm-469/SYM.ROM"
printf 'dll' > "$data/drives/z/rm-469/sys/bin/avkonfep.dll.bak"
printf 'RM-469:\n  platver: epoc93fp2\n  firmcode: RM-469\n  model: N00\nRM-1:\n  firmcode: RM-1\n' > "$data/devices.yml"
before=$(cd "$data" && find . -type f -exec sha256sum {} + | sort)

EKA2L1_DATA=$data bash "$stage" "$tmp/out" > "$tmp/out.log" 2>&1
check "the ROM is staged" cmp -s "$data/roms/rm-469/SYM.ROM" "$tmp/out/roms/rm-469/SYM.ROM"
check "drive Z is staged with its z: folder" test -d "$tmp/out/drives/z/rm-469/z:/private"
check "device.yml is the RM-469 entry" sh -c "printf 'RM-469:\n  platver: epoc93fp2\n  firmcode: RM-469\n  model: N00\n' | cmp -s - '$tmp/out/device.yml'"
check "drive C is not staged" test ! -e "$tmp/out/drives/c"
check "the data folder is unchanged" sh -c "[ \"\$(cd '$data' && find . -type f -exec sha256sum {} + | sort)\" = '$before' ]"
check "an existing output folder is refused" sh -c "! EKA2L1_DATA='$data' bash '$stage' '$tmp/out' 2>/dev/null"
check "no EKA2L1_DATA is refused" sh -c "! env -u EKA2L1_DATA bash '$stage' '$tmp/out2' 2>/dev/null"
rm -r "$data/roms/rm-469"
check "a missing ROM is refused by name" sh -c "EKA2L1_DATA='$data' bash '$stage' '$tmp/out3' 2>&1 | grep -q 'roms/rm-469 is missing'"
[ "$failures" -eq 0 ] || { echo "$failures failed"; exit 1; }
```

Run `sh tests/firmware-stage.test`. Expected: it fails, because `stage.sh` does not exist.

- [ ] **Step 6: Write `recipes/firmware/rm-469/1/stage.sh`**

```bash
#!/usr/bin/env bash
# Stage firmware;rm-469;1 from an EKA2L1 data folder, for `publish private`:
#
#   EKA2L1_DATA=~/.local/share/EKA2L1/data bash stage.sh <out-dir>
#
# <out-dir> must not exist. It gets roms/rm-469/ and drives/z/rm-469/ copied as they are,
# and device.yml, the RM-469 entry of $EKA2L1_DATA/devices.yml (pkgtools device-entry).
# Nothing in EKA2L1_DATA is written. PKGTOOLS names a pkgtools binary; by default this
# repository's is run through cargo.
set -euo pipefail
if [ $# -ne 1 ]; then
  echo "usage: EKA2L1_DATA=<EKA2L1 data folder> stage.sh <out-dir>" >&2
  exit 2
fi
data=${EKA2L1_DATA:?set EKA2L1_DATA to the EKA2L1 data folder that holds devices.yml, roms/ and drives/}
out=$1
here=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
pkgtools=${PKGTOOLS:-"cargo run --release --quiet --manifest-path $here/../../../../Cargo.toml -p pkgtools --"}
if [ -e "$out" ]; then
  echo "error: $out exists; stage.sh writes a new folder" >&2
  exit 1
fi
for part in devices.yml roms/rm-469 drives/z/rm-469; do
  if [ ! -e "$data/$part" ]; then
    echo "error: $data/$part is missing; install the RM-469 firmware in EKA2L1 first" >&2
    exit 1
  fi
done
mkdir -p "$out/roms" "$out/drives/z"
$pkgtools device-entry "$data/devices.yml" RM-469 > "$out/device.yml"
cp -a "$data/roms/rm-469" "$out/roms/"
cp -a "$data/drives/z/rm-469" "$out/drives/z/"
echo "staged firmware;rm-469;1 in $out"
```

`sh tests/firmware-stage.test` now prints only `ok` lines. `.github/workflows/tests.yml`
already triggers on `recipes/**` and `tests/**`; add a step after the install.sh ones:

```yaml
      - name: The firmware recipe's stage.sh
        run: sh tests/firmware-stage.test
```

- [ ] **Step 7: Write the recipe and pin its archive**

`recipes/firmware/rm-469/1/recipe.toml`:

```toml
# firmware;rm-469;1 — the Nokia E52's firmware (RM-469) in EKA2L1's data layout, which
# symdev makes emulator profiles from (symdev's 2026-10-03-emulator-firmware-packages-design
# §4): roms/rm-469/SYM.ROM, drives/z/rm-469/ and device.yml, the RM-469 entry of EKA2L1's
# devices.yml (platver epoc93fp2, firmcode RM-469, model N00, …).
#
# The bytes are Nokia's, with no known redistribution grant. They live only on the owner's
# machine and in the private bucket: never in this repository, in CI or in the public
# bucket (the publisher refuses a LicenseRef- licence there). The owner stages and
# publishes them:
#   EKA2L1_DATA=~/.local/share/EKA2L1/data bash recipes/firmware/rm-469/1/stage.sh <tree>
#   cargo run --release -p publish -- private 'firmware;rm-469;1' --from <tree> \
#     --recipe recipes/firmware/rm-469/1/recipe.toml
#
# Drive Z is in the state EKA2L1 leaves it after the first start: Z:\sys\bin\avkonfep.dll
# moved to avkonfep.dll.bak (symdev experiment 115 §1.3).
id = "firmware;rm-469;1"
license = "LicenseRef-Nokia-firmware"
host = "any"
include = ["device.yml", "roms/rm-469", "drives/z/rm-469"]
```

Then, from the scratch folder (the dry run writes the 135 MB archive into the current
directory):

```bash
mkdir -p ~/src/emu-pkg-scratch/firmware && cd ~/src/emu-pkg-scratch/firmware && rm -rf tree
P=~/worktrees/symdev-packages/cargo-run
EKA2L1_DATA=~/.local/share/EKA2L1/data bash $P/recipes/firmware/rm-469/1/stage.sh tree
env -u PUBLISH_PRIVATE_URL cargo run --release --quiet --manifest-path $P/Cargo.toml -p publish -- \
  private 'firmware;rm-469;1' --from tree --recipe $P/recipes/firmware/rm-469/1/recipe.toml --dry-run
```

Expected: an error that ends with ``record `sha256 = "<64 hex>"` in …recipe.toml and run
again``. Add that line to the recipe. Above it, add a comment with the date and the
`packed …` line's byte count and the file count (`find tree -type f | wc -l`). Run the dry run
again; expected: `packed firmware;rm-469;1: …` and the index printed with
`license = "LicenseRef-Nokia-firmware"`. Then check that the public bucket refuses it:

```bash
env -u PUBLISH_PUBLIC_URL cargo run --release --quiet --manifest-path $P/Cargo.toml -p publish -- \
  public 'firmware;rm-469;1' --from tree --source-code /dev/null \
  --recipe $P/recipes/firmware/rm-469/1/recipe.toml --dry-run
```

Expected: `… grants no right to publish it; only the private bucket may hold it`.

- [ ] **Step 8: Gates and commit** (packages worktree)

```bash
cd ~/worktrees/symdev-packages/cargo-run
cargo test --locked > /tmp/t5-all.log 2>&1; grep -E "FAILED|^error" /tmp/t5-all.log
cargo clippy --all-targets --locked > /tmp/t5-clippy.log 2>&1; grep -cE "^(warning|error)" /tmp/t5-clippy.log
cargo fmt --all --check && sh tests/firmware-stage.test | grep -c '^not ok'
git add pkgtools/src/device_entry.rs pkgtools/src/main.rs pkgtools/tests/cli.rs \
  recipes/firmware/rm-469/1/recipe.toml recipes/firmware/rm-469/1/stage.sh \
  tests/firmware-stage.test .github/workflows/tests.yml
git commit -m "Add the private firmware;rm-469;1 recipe, staged from an EKA2L1 data folder and pinned to the owner's archive."
```

Expected: no failure lines, `0` clippy lines, `0` `not ok`. A developer's
`PUBLISH_SIGNING_KEY` in the environment makes one existing publisher test fail (cargo-run
notes); run the gate with `env -u PUBLISH_SIGNING_KEY`. Leave `~/src/emu-pkg-scratch/firmware/tree`
for Task 6. Commit the symdev wip file too.

### Task 6: A profile made from a read-only firmware package (experiment 115 §3)

The spec: "Whether EKA2L1 writes into Z: or the ROM is observed first, with the package files
read-only; if it writes, the profile gets copies instead." It also leaves open whether the
firmware boots with an empty drive C and a `config.yml` holding only symdev's log filter.
This task answers all three before Task 7 writes the code. It uses Task 1's host build, so
that only the firmware side is new.

**Files:**
- Outside git: `~/src/emu-pkg-scratch/exp115/{pkg,profile,probe.py,run6.sh,run6.log,strace.txt}`
- Modify: `docs/research/experiment-backlog.md` (experiment 115 §3)

**Interfaces:**
- Consumes: Task 5's staged tree, Task 1's `eka2l1-emupkg`.
- Produces: the ruling **links** or **copies** for Task 7, and whether an empty C boots.

- [ ] **Step 1: Make the package and the profile by hand**

```bash
X=~/src/emu-pkg-scratch/exp115; rm -rf $X/pkg $X/profile; mkdir -p $X/pkg $X/profile/data/roms
cp -a ~/src/emu-pkg-scratch/firmware/tree $X/pkg/rm-469 && chmod -R a-w $X/pkg/rm-469
P=$X/pkg/rm-469; D=$X/profile/data
mkdir -p $D/drives/c $D/drives/d $D/drives/e
cp $P/device.yml $D/devices.yml
ln -s $P/roms/rm-469 $D/roms/rm-469
ln -s $P/drives/z $D/drives/z
printf 'log-filter: "*:info Emulated.Stdout:trace Kernel:trace"\n' > $X/profile/config.yml
cp ~/src/cargo-run-scratch/exec/probe10.py $X/probe.py
```

`$P/drives/z` is the folder that holds `rm-469/`; EKA2L1 finds Z under
`data/drives/z/<firmware>`, as in the owner's layout. That layout is why
`EmulatorProfile::create` links the whole `drives/z` today.

- [ ] **Step 2: Run it under strace and the agent lock**

`run6.sh`:

```bash
#!/usr/bin/env bash
# run6.sh — experiment 115 §3: Task 1's EKA2L1 on a profile made from the read-only firmware
# package; every file call traced; hello installed and launched through the control socket.
X=~/src/emu-pkg-scratch/exp115; S=$XDG_RUNTIME_DIR/symdev-exp115.sock; rm -f $S
touch $X/marker
setsid strace -f -e trace=%file -o $X/strace.txt ~/src/emu-pkg-scratch/bin/eka2l1-emupkg \
  --data-dir $X/profile --control $S > $X/emu.out 2>&1 & pid=$!
echo "pid $pid"
python3 $X/probe.py $S ~/src/cargo-run-scratch/tree/symbian-rs/examples/hello/build/hello.sisx 0xef9f2cab $X/shots
sleep 2; kids=$(pgrep -P $pid); kill -9 $kids $pid; sleep 1
for p in $pid $kids; do kill -0 $p 2>/dev/null && echo "still alive: $p"; done
grep -E "$X/pkg" $X/strace.txt | grep -E 'O_WRONLY|O_RDWR|O_CREAT|rename|unlink|mkdir' > $X/pkg-writes.txt
echo "write attempts on the package: $(wc -l < $X/pkg-writes.txt)"
find -L $X/pkg -newer $X/marker | head
grep -iE 'error|fail|denied' $X/profile/EKA2L1.log | head -20
ls -R $X/profile/data/drives/c | head -30
```

```bash
flock ~/.local/share/EKA2L1/.symdev-agent.lock bash ~/src/emu-pkg-scratch/exp115/run6.sh > ~/src/emu-pkg-scratch/exp115/run6.log 2>&1
```

The script kills only the PID it started (strace) and that PID's children (EKA2L1),
listed before strace dies. Once strace dies, its children belong to init and `pgrep -P`
would no longer find them. The owner's own emulator is never matched. `run6.log` must
have no `still alive` line.

- [ ] **Step 3: Read the answers and rule**

From `run6.log`, `probe.py`'s JSON lines, `strace.txt` and `EKA2L1.log`:

1. **Boots with empty C and a one-line config?** `emulator.info` names `RM-469` and
   `apps.list` answers. If not, record the last log lines and stop: Task 7 cannot be written
   without the answer, so report to the lead (the fix is a seed for C in the firmware
   package, which changes the recipe).
2. **Write attempts into the package** (`pkg-writes.txt`). Expected from §1.3: one
   failing open of `…/sys/bin/avkonfep.dll` for writing (the backslash copy may not even
   reach it), and no rename, because the staged Z already holds the `.bak`. Each line is
   recorded.
3. **Did a write attempt change behaviour?** `package.install` of hello answers `{}`, the
   launch gives a pid, `event.app_exited` comes with `exit_type kill`, and the log shows no
   error on Z or the ROM beyond the known avkonfep copy.

**Ruling:** if 1 and 3 hold, Task 7 uses **links** (as written). If anything in 3 fails
because Z or the ROM is read-only, Task 7 uses **copies**: the variant given in Task 7
step 3. The profile then costs 259 MB each, and the rest of the plan is unchanged. Write
the ruling, with the evidence, as experiment 115 §3, and into the wip file.

- [ ] **Step 4: Commit** the experiment record and the wip file:
  `Record experiment 115 §3: a profile made from a read-only firmware package.`
  Restore write permission on the scratch copy afterwards (`chmod -R u+w
  ~/src/emu-pkg-scratch/exp115/pkg`) so that later `rm -rf` works.

### Task 7: Profiles made from a firmware package

**Files:**
- Create: `crates/symdev-emulator/src/device/firmware.rs` (`Firmware`)
- Modify: `crates/symdev-emulator/src/device.rs` (`mod firmware; pub use firmware::Firmware;`)
- Modify: `crates/symdev-emulator/src/device/emulator_profile.rs` (`create(&Firmware)`, `check()`)
- Modify: `crates/symdev-emulator/src/device/emulator_instance.rs` (`start` calls `check` first)
- Modify: `crates/symdev-emulator/src/results.rs` (delete `EmulatorData::from_env`)
- Create: `crates/symdev-emulator/src/device/tests/firmware.rs`; Modify: `device/tests.rs`
  (`mod firmware;`), `device/tests/profile.rs` (the three `create` calls)
- Modify: `crates/symdev-cli/src/devices_cmd.rs` (only so the workspace builds: see step 4)

**Interfaces:**
- Consumes: Task 6's ruling (links or copies).
- Produces:
  - `Firmware::UserData { data: EmulatorData, name: String }`,
    `Firmware::Package { root: PathBuf, name: String }`, `Firmware::name(&self) -> &str`,
    `Firmware::in_user_data(data: &EmulatorData) -> Vec<Firmware>`.
  - `EmulatorProfile::create(&self, firmware: &Firmware) -> Result<()>`,
    `EmulatorProfile::check(&self) -> Result<()>`.
  - `EmulatorData::from_env` no longer exists. The user's data is reached only through
    `SYMDEV_EKA2L1_DATA`, which `Provision` reads (Task 9).

- [ ] **Step 1: Write the failing tests**, `crates/symdev-emulator/src/device/tests/firmware.rs`

```rust
use std::path::Path;

use crate::EmulatorData;
use crate::device::{DeviceRegistry, EmulatorInstance, EmulatorProfile, Firmware};

/// An installed `firmware;rm-469;1` as Task 5's stage.sh makes it.
fn package(root: &Path) -> Firmware {
    for d in ["roms/rm-469", "drives/z/rm-469/sys/bin"] {
        std::fs::create_dir_all(root.join(d)).unwrap();
    }
    std::fs::write(root.join("roms/rm-469/SYM.ROM"), b"rom").unwrap();
    std::fs::write(root.join("device.yml"), "RM-469:\n  firmcode: RM-469\n").unwrap();
    Firmware::Package {
        root: root.to_path_buf(),
        name: "rm-469".into(),
    }
}

#[test]
fn a_profile_from_a_package_links_its_rom_and_drive_z_and_starts_with_empty_drives() {
    let pkg = tempfile::tempdir().unwrap();
    let root = tempfile::tempdir().unwrap();
    let p = EmulatorProfile::at(root.path(), "rm-469");
    p.create(&package(pkg.path())).unwrap();
    let data = p.dir().join("data");
    assert_eq!(std::fs::read_link(data.join("roms/rm-469")).unwrap(), pkg.path().join("roms/rm-469"));
    assert_eq!(std::fs::read_link(data.join("drives/z")).unwrap(), pkg.path().join("drives/z"));
    for d in ["c", "d", "e"] {
        let drive = data.join("drives").join(d);
        assert!(drive.is_dir() && std::fs::read_dir(&drive).unwrap().next().is_none(), "{d}");
    }
}

#[test]
fn a_profile_from_a_package_lists_the_packages_device_and_only_symdevs_log_filter() {
    let pkg = tempfile::tempdir().unwrap();
    let root = tempfile::tempdir().unwrap();
    let p = EmulatorProfile::at(root.path(), "rm-469");
    p.create(&package(pkg.path())).unwrap();
    assert_eq!(
        std::fs::read_to_string(p.dir().join("data/devices.yml")).unwrap(),
        "RM-469:\n  firmcode: RM-469\n"
    );
    assert_eq!(
        std::fs::read_to_string(p.dir().join("config.yml")).unwrap(),
        "log-filter: \"*:info Emulated.Stdout:trace Kernel:trace\"\n"
    );
}

#[test]
fn a_profile_whose_package_is_gone_is_refused_before_start() {
    let pkg = tempfile::tempdir().unwrap();
    let root = tempfile::tempdir().unwrap();
    let run = tempfile::tempdir().unwrap();
    let p = EmulatorProfile::at(root.path(), "rm-469");
    p.create(&package(pkg.path())).unwrap();
    assert!(p.check().is_ok());
    let gone = pkg.path().to_path_buf();
    drop(pkg);
    let e = p.check().unwrap_err().to_string();
    assert!(e.contains("emulator profile rm-469"), "{e}");
    assert!(e.contains(&gone.join("drives/z").display().to_string()), "{e}");
    assert!(e.contains(&p.dir().display().to_string()), "{e}");
    let registry = DeviceRegistry::at(run.path().join("devices"));
    let start = EmulatorInstance::start(Path::new("/nonexistent/eka2l1"), &p, &registry)
        .unwrap_err()
        .to_string();
    assert_eq!(start, e, "start must refuse before it runs anything");
}

#[test]
fn the_users_firmwares_are_the_folders_of_data_roms_by_name() {
    let user = tempfile::tempdir().unwrap();
    for f in ["rm-469", "rm-356"] {
        std::fs::create_dir_all(user.path().join("data/roms").join(f)).unwrap();
    }
    let data = EmulatorData::at(user.path());
    let names: Vec<String> = Firmware::in_user_data(&data)
        .iter()
        .map(|f| f.name().to_string())
        .collect();
    assert_eq!(names, ["rm-356", "rm-469"]);
    assert!(Firmware::in_user_data(&EmulatorData::at(&user.path().join("none"))).is_empty());
}
```

In `device/tests/profile.rs`, the three calls `create(&EmulatorData::at(X), "rm-469")`
become `create(&Firmware::UserData { data: EmulatorData::at(X), name: "rm-469".into() })`
(import `crate::device::Firmware`). Their assertions stay as they are: the user-data route
does not change.

- [ ] **Step 2: Run them and see them fail**

```bash
cargo test -p symdev-emulator --offline device:: > /tmp/t7.log 2>&1; grep -E "^error|test result" /tmp/t7.log
```

Expected: `unresolved import crate::device::Firmware` and `no method named check`.

- [ ] **Step 3: Implement**

`crates/symdev-emulator/src/device/firmware.rs`:

```rust
//! `Firmware`: what an emulator profile is made from (emulator packages spec §5): a
//! firmware installed in the user's EKA2L1 (reached through `SYMDEV_EKA2L1_DATA`), or an
//! installed `firmware;<name>;<n>` package.
use std::path::PathBuf;

use crate::EmulatorData;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Firmware {
    /// `data/roms/<name>` of the user's EKA2L1 data folder; drive Z, drive C,
    /// `devices.yml` and `config.yml` are that folder's.
    UserData { data: EmulatorData, name: String },
    /// An installed package: `roms/<name>/`, `drives/z/<name>/` and `device.yml` under `root`.
    Package { root: PathBuf, name: String },
}

impl Firmware {
    /// The firmware's folder name, which is also its profile's name (`rm-469`).
    pub fn name(&self) -> &str {
        match self {
            Firmware::UserData { name, .. } | Firmware::Package { name, .. } => name,
        }
    }

    /// Every firmware installed in the user's data folder `data` (`data/roms/<name>/`), in
    /// name order; none when the folder has no `data/roms`.
    pub fn in_user_data(data: &EmulatorData) -> Vec<Firmware> {
        let roms = data.root().join("data/roms");
        let mut names: Vec<String> = std::fs::read_dir(&roms)
            .map(|dir| {
                dir.flatten()
                    .filter(|e| e.path().is_dir())
                    .filter_map(|e| e.file_name().into_string().ok())
                    .collect()
            })
            .unwrap_or_default();
        names.sort();
        names
            .into_iter()
            .map(|name| Firmware::UserData {
                data: data.clone(),
                name,
            })
            .collect()
    }
}
```

`emulator_profile.rs`: today's `create(from, firmware)` body below the "already exists"
check becomes `fn create_from_user(&self, from: &EmulatorData, firmware: &str) ->
Result<()>`, unchanged. Then:

```rust
    /// Makes the profile from `firmware`; an existing profile is never overwritten. From
    /// the user's data: ROM and the whole drive Z linked, `devices.yml`, drive C and
    /// `config.yml` copied (experiment 114 §2). From a package: ROM and drive Z linked
    /// into it, its `device.yml` as `devices.yml`, empty C, D and E, and a `config.yml`
    /// holding only symdev's log filter, every other option at EKA2L1's default
    /// (experiment 115 §3).
    pub fn create(&self, firmware: &Firmware) -> Result<()> {
        if self.dir.symlink_metadata().is_ok() {
            return Err(Error::Other(format!(
                "emulator profile {} already exists at {}",
                self.name,
                self.dir.display()
            )));
        }
        match firmware {
            Firmware::UserData { data, name } => self.create_from_user(data, name),
            Firmware::Package { root, name } => self.create_from_package(root, name),
        }
    }

    fn create_from_package(&self, root: &Path, name: &str) -> Result<()> {
        let data = self.dir.join("data");
        for dir in ["drives/c", "drives/d", "drives/e", "roms"] {
            make_dir(&data.join(dir))?;
        }
        copy_file(&root.join("device.yml"), &data.join("devices.yml"))?;
        link(&root.join("roms").join(name), &data.join("roms").join(name))?;
        link(&root.join("drives/z"), &data.join("drives/z"))?;
        let out = self.dir.join("config.yml");
        std::fs::write(&out, format!("{LOG_FILTER}\n")).map_err(|e| file(&out, e))
    }

    /// Refuses a profile whose ROM or drive Z is a link to nothing (its firmware package
    /// was uninstalled, or the user's EKA2L1 data moved), before EKA2L1 starts on it.
    pub fn check(&self) -> Result<()> {
        let data = self.dir.join("data");
        let mut links = vec![data.join("drives/z")];
        if let Ok(roms) = std::fs::read_dir(data.join("roms")) {
            links.extend(roms.flatten().map(|e| e.path()));
        }
        for path in links {
            let is_link = path.symlink_metadata().is_ok_and(|m| m.file_type().is_symlink());
            if is_link && !path.exists() {
                let target = std::fs::read_link(&path).map_err(|e| file(&path, e))?;
                return Err(Error::Other(format!(
                    "emulator profile {} links {} to {}, which is gone: its firmware package \
                     was uninstalled or its EKA2L1 data moved. Install it again (`symdev sdk \
                     install`), or remove {} and symdev makes the profile again",
                    self.name,
                    path.display(),
                    target.display(),
                    self.dir.display()
                )));
            }
        }
        Ok(())
    }
```

**If Task 6 ruled copies:** in `create_from_package`, replace the two `link` lines with
`copy_tree(&root.join("roms"), &root.join("roms").join(name), &data.join("roms").join(name))?;`
and `copy_tree(&root.join("drives/z"), &root.join("drives/z"), &data.join("drives/z"))?;`.
Change the first test's two `read_link` assertions to `is_dir()` plus one copied file
compared with `std::fs::read`. Delete `check()`, its call and its test, which then guard
nothing, and drop Review Focus item 3 with a note in the wip file.

`emulator_instance.rs`, first line of `start`: `profile.check()?;`.

`results.rs`: delete `EmulatorData::from_env` and its doc comment. Rewrite the struct's doc so
that it no longer names a default: "EKA2L1's data folder: a profile's own, or the user's
named by `SYMDEV_EKA2L1_DATA`". The file then has no `std::env` use. Keep the imports
clippy still needs.

- [ ] **Step 4: Keep the CLI building**

`devices_cmd.rs`'s `profiles()` called `EmulatorData::from_env()`. Until Task 9 wires
`Provision`, replace those lines with the equivalent that reads the variable in the CLI:

```rust
        let user = match std::env::var_os("SYMDEV_EKA2L1_DATA").filter(|v| !v.is_empty()) {
            Some(dir) => EmulatorData::at(std::path::Path::new(&dir)),
            None => return Ok(Vec::new()),
        };
        for firmware in Firmware::in_user_data(&user) {
            self.profile(firmware.name()).create(&firmware)?;
            eprintln!("created profile {}", firmware.name());
        }
```

This is temporary; Task 9 replaces it. `crates/symdev-cli/tests/run.rs`'s
`run_with_a_profile_to_start_and_no_emulator_names_symdev_eka2l1` now sets
`SYMDEV_EKA2L1_DATA` to `user` (its `XDG_DATA_HOME/EKA2L1`) instead of relying on the default
folder. Every other CLI test that made a firmware under `XDG_DATA_HOME/EKA2L1` does the same:
`grep -rln 'EKA2L1' crates/symdev-cli/tests` lists them, among them `tests/test_cmd.rs`
and `tests/common/fake_control.rs`.

- [ ] **Step 5: Run the tests and the gates** (Task 4 step 4's three commands). Expected: all
  pass, 0 clippy lines.

- [ ] **Step 6: Commit**

```bash
git add crates/symdev-emulator/src/device.rs crates/symdev-emulator/src/device/firmware.rs \
  crates/symdev-emulator/src/device/emulator_profile.rs crates/symdev-emulator/src/device/emulator_instance.rs \
  crates/symdev-emulator/src/results.rs crates/symdev-emulator/src/device/tests.rs \
  crates/symdev-emulator/src/device/tests/firmware.rs crates/symdev-emulator/src/device/tests/profile.rs \
  crates/symdev-cli/src/devices_cmd.rs crates/symdev-cli/tests
git commit -m "Make an emulator profile from a firmware package, refuse one whose firmware is gone, and read the user's EKA2L1 data only through SYMDEV_EKA2L1_DATA."
```
