# symdev

Rust CLI toolchain that builds, packages, signs and runs Symbian S60 3rd FP2 apps (Nokia E52) from Linux, replacing the SDK's Windows tools with native code.

Reply in the user's language (usually Ukrainian).

## How to work

**Own the outcome.** Do the work: drive the GUI, logs and config yourself instead of asking the user to click, relaunch or "try it". When the next step is obvious, do it. A crash you narrate is not progress — fix it or name the next action you will take.

**Verify before "ok".** Never claim it works from code, a log line or a green patch alone. Check what the user sees: window, pixels, file installed, process still alive. Use the `superpowers:verification-before-completion` skill before saying done.

**Fix causes, not symptoms.** A threshold, magic skip or silenced check that hides a symptom is not a fix. If you must ship a stopgap, label it as one in the code and in the report.

**Treat earlier claims as hypotheses.** Commit messages, reports and previous sessions (including Cursor's, whose transcripts are at `~/.cursor/projects/home-genius-*symdev*/agent-transcripts/*/*.jsonl`) are unverified until you rebuild and rerun the tests yourself.

**Finish the previous task.** A new message does not close the last one: record the unfinished work, do what was asked, then come back.

**Ask only about decisions that are the user's** — conflicting requirements, a `sudo` password, anything outward-facing. Otherwise keep going.

Use the `superpowers:*` skills actively: brainstorming before design, test-driven-development while implementing, requesting-code-review before merging, writing-plans for multi-step work.

## Workflow

- Rust 1.98.1, edition 2024, resolver 3. Before reporting done: `cargo test --workspace --offline` and `cargo clippy --workspace --all-targets --offline`, both clean (zero warnings since 2026-09-19).
- Feature work: a branch in a worktree under `~/worktrees/symdev/<branch>`, merged into `main`.
- Commit messages: one full imperative sentence ending with a period (see `git log`).
- Research notes and experiment records: `docs/research/`. Specs and plans: `docs/superpowers/`. Task briefs: `.superpowers/sdd/`.

## Code

- Library paths return `Result`; no `unwrap` / `expect` / `panic!` outside tests. An error names what failed and, when known, the fix.
- Types own behaviour: new API is a domain type with methods (`UidCrc::checked`, `SisPackage::package`), not a module of `pub fn`. Value types never see Wine, env, argv or stdout — that lives on `*Tool` / `*Backend` adapters.
- Public API stays small and named after domain concepts. Delete unused helpers in the same change.
- Every `.rs` file is at most 300 lines, tests included; split by domain concept, one type per file, `<module>/tests.rs` for long test modules.
- Behaviour that was never observed from the original tools is an error (`TODO: … (not observed)`), never a guess. Full playbook: the `symbian-formats` skill.

## Host

This Linux machine is the host: build here, no Windows VM. Never commit or download SDK, ROM, `.sis`, `.sisx`, `.cer`, `.key`. Do not invent tool argv — only flags observed from the real tools. Do not claim E52 support until a stock device installs and launches the app; the emulator is not a device.

SDK and toolchain come from the environment: `SYMDEV_EPOCROOT`, `SYMDEV_GXX`, `SYMDEV_LD`, `SYMDEV_GCC_LIB`, `SYMDEV_GCC_TARGET_LIB`, `SYMDEV_WINE` (only `mifconv` for icons), `SYMDEV_EKA2L1`, `SYMDEV_SIGN_PASSWORD`.

EKA2L1 runs as a separate process (it is GPL-3.0; never copy its source in). It ignores SIGTERM: stop only instances you started, with `kill -9 <pid>`, and never blanket-kill — the user may have one open. Emulator source, patches, rebuild and the screenshot loop: the `eka2l1-host` skill.
