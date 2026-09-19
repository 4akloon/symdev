# Agent behavior

How to work on this repo. Applies to every agent (Cursor, Claude Code, others).
Code style lives in `.cursor/rules/*.mdc` — do not duplicate it here. `CLAUDE.md` imports those files for Claude Code.

## Own the outcome

Do the work. Do not wait for the user to click, relaunch, install, or “try it”. Drive the GUI, logs, and config yourself.

When the next step is obvious, apply it immediately. Do not explain the fix and stop.

Do not dump errors as the deliverable. Fix them, or give a concrete next action you will take. Narrating a crash is not progress.

## Verify before “ok”

Never claim it works from code, a checkbox, or a log line alone.

Check the thing the user sees: window, list, pixels, process still alive, file actually installed. If the screenshot is black or the UI still looks wrong, it is not done.

## Fix causes, not symptoms

A heuristic that hides a symptom (size thresholds, magic skips, silenced checks) is not a fix. Find the cause. If you must ship a stopgap, label it as one in code and in the report, and say what the real fix is.

## Handoffs between agents

Claims from a previous session (transcripts, `.superpowers/sdd/*-report.md`, commit messages, “it works now”) are hypotheses, not facts. Before building on them:

- rebuild and rerun the tests yourself;
- read the uncommitted diff, not just the summary;
- mark which bytes are pinned by a golden and which are hand-built without one.

## Finish the previous task

A new message does not close the last one. If the user interrupts, record the unfinished work, do what they asked, then resume the previous task until it is verified done.

## Initiative

Continue the plan. Do not stop to ask “what next?” after a green patch. Use subagents and worktrees; run independent tasks in parallel when you can.

Call the user only for a `sudo` password or a decision only they can make.

## Decisions and prompts

Ask about conflict decisions. A new prompt is not total truth — lock conflicts with the user, then implement from those locks.

Cursor: do not use fast models; prefer `cursor-grok-4.6-high`.

## Host

This Linux machine is the host: edit + build here. No Windows VM. macOS later, not this iteration.

Do not invent argv. Do not curl, vendor, or commit SDK, ROM, `.sis`, `.sisx`, `.cer`, `.key`.

Never set `MESA_GL_VERSION_OVERRIDE`.

Do not claim E52 support until stock install + launch on hardware. Emulator success is not device support.

## Workflow

- Toolchain: Rust **1.98.1 / edition 2024 / resolver 3**. Offline builds: `cargo test --workspace --offline`, `cargo clippy --workspace --offline`.
- Feature work happens on a branch in a worktree under `~/worktrees/symdev/<branch>`, merged into `main`.
- Specs and plans: `docs/superpowers/specs/`, `docs/superpowers/plans/`. Research notes: `docs/research/`. Task briefs/reports: `.superpowers/sdd/`.
- Commit messages: one full sentence, imperative, ending with a period (see `git log`).

## Host state outside the repo

The emulator bring-up lives outside git — check it before assuming anything:

- EKA2L1 source `~/src/EKA2L1` (+ local diff), exported patch `~/src/EKA2L1-econs-heap.patch`, build dir `~/src/EKA2L1-build`.
- Runs as systemd user unit `eka2l1-patched.service`; config `~/.local/share/EKA2L1/config.yml`, `~/.config/EKA2L1/EKA2L1.conf`.
- `SYMDEV_EKA2L1=~/.local/bin/eka2l1-patched` (wrapper with the service env) for `symdev run`.
- Rebuild: `PATH=~/.local/eka2l1-tools/bin:~/.local/eka2l1-tools/cmake/bin:$PATH LIBRARY_PATH=~/.local/eka2l1-sysroot/usr/lib/x86_64-linux-gnu ninja -C ~/src/EKA2L1-build eka2l1_qt ekatests`; run `ekatests` from `~/src/EKA2L1-build/src/tests`. `eka2l1_qt --run hello` launches the app without clicking.
- Re-export the patch after changing the emulator: `git -C ~/src/EKA2L1 diff > ~/src/EKA2L1-econs-heap.patch` (new files need `git add -N` first; the build rewrites `src/emu/qt/translations/*.ts` — `git checkout` them before exporting).
- Notes: `docs/research/eka2l1-bringup.md`, `docs/research/eka2l1.md`.

## Code (details in `.cursor/rules/`)

Types own behavior. No new piles of bare `pub fn`. Value types do not know Wine, env, or argv — adapters (`*Tool`) do.

Library `Result`, no `unwrap` in non-test library code.
