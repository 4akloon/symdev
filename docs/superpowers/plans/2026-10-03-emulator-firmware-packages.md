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

`crates/symdev-sdk/src/emulator_package.rs` gets a test module like `platform_sdk.rs`'s:

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
`file not found for module`, `no associated item named ALL`). libtest takes several
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

`lib.rs`: `mod emulator_package;`, `mod firmware_package;`, `pub use
emulator_package::EmulatorPackage;`, `pub use firmware_package::FirmwarePackage;`, in the
alphabetical places of the existing lists.

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
